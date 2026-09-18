package main

import (
	"bytes"
	"encoding/json"
	"flag"
	"io"
	"net/http"
	"os"
	"sort"
	"sync"
	"sync/atomic"
	"time"
)

type BenchResult struct {
	TotalReqs   int     `json:"total_req"`
	SuccessReqs int     `json:"success_req"`
	FailReqs    int     `json:"fail_req"`
	ReqPerSec   float64 `json:"req_per_sec"`
	AvgMs       float64 `json:"avg_ms"`
	P50Ms       float64 `json:"p50_ms"`
	P90Ms       float64 `json:"p90_ms"`
	P99Ms       float64 `json:"p99_ms"`
	MaxMs       float64 `json:"max_ms"`
	DurationSec float64 `json:"duration_sec"`
}

func main() {
	baseURL := flag.String("base", "http://127.0.0.1:8000", "base URL")
	method := flag.String("method", "GET", "HTTP method")
	path := flag.String("path", "/api/v1/clusters", "request path")
	body := flag.String("body", "", "request body (inline)")
	bodyFile := flag.String("bodyfile", "", "request body file path (overrides -body)")
	token := flag.String("token", "", "bearer token")
	concurrency := flag.Int("c", 10, "concurrency")
	duration := flag.Int("d", 20, "duration seconds")
	flag.Parse()

	if *bodyFile != "" {
		data, err := os.ReadFile(*bodyFile)
		if err != nil {
			panic(err)
		}
		*body = string(data)
	}

	client := &http.Client{
		Timeout: 10 * time.Second,
		Transport: &http.Transport{
			MaxIdleConns:        200,
			MaxIdleConnsPerHost: 200,
			MaxConnsPerHost:    200,
			IdleConnTimeout:    30 * time.Second,
		},
	}

	uri := *baseURL + *path
	deadline := time.Now().Add(time.Duration(*duration) * time.Second)

	var totalReqs int64
	var failReqs int64
	latencies := make([]float64, 0, 100000)
	var latMu sync.Mutex

	var wg sync.WaitGroup
	for i := 0; i < *concurrency; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for time.Now().Before(deadline) {
				atomic.AddInt64(&totalReqs, 1)
				reqStart := time.Now()

				var reqBody io.Reader
				if *body != "" {
					reqBody = bytes.NewBufferString(*body)
				}

				req, err := http.NewRequest(*method, uri, reqBody)
				if err != nil {
					atomic.AddInt64(&failReqs, 1)
					continue
				}
				if *body != "" {
					req.Header.Set("Content-Type", "application/json")
				}
				req.Header.Set("Accept", "application/json")
				if *token != "" {
					req.Header.Set("Authorization", "Bearer "+*token)
				}

				resp, err := client.Do(req)
				reqDur := time.Since(reqStart)

				if err != nil {
					atomic.AddInt64(&failReqs, 1)
					continue
				}
				io.Copy(io.Discard, resp.Body)
				resp.Body.Close()

				if resp.StatusCode >= 400 {
					atomic.AddInt64(&failReqs, 1)
				}

				latMu.Lock()
				latencies = append(latencies, float64(reqDur.Milliseconds()))
				latMu.Unlock()
			}
		}()
	}

	wg.Wait()

	// Compute statistics
	total := int(atomic.LoadInt64(&totalReqs))
	fails := int(atomic.LoadInt64(&failReqs))
	success := total - fails

	var avg float64
	if len(latencies) > 0 {
		sum := 0.0
		for _, l := range latencies {
			sum += l
		}
		avg = sum / float64(len(latencies))
	}

	sort.Float64s(latencies)
	pctile := func(p float64) float64 {
		if len(latencies) == 0 {
			return 0
		}
		idx := int(p / 100.0 * float64(len(latencies)))
		if idx >= len(latencies) {
			idx = len(latencies) - 1
		}
		return latencies[idx]
	}

	max := 0.0
	if len(latencies) > 0 {
		max = latencies[len(latencies)-1]
	}

	durSec := float64(*duration)
	throughput := 0.0
	if durSec > 0 {
		throughput = float64(total) / durSec
	}

	result := BenchResult{
		TotalReqs:   total,
		SuccessReqs: success,
		FailReqs:    fails,
		ReqPerSec:   throughput,
		AvgMs:       avg,
		P50Ms:       pctile(50),
		P90Ms:       pctile(90),
		P99Ms:       pctile(99),
		MaxMs:       max,
		DurationSec: durSec,
	}

	json.NewEncoder(os.Stdout).Encode(result)
}
