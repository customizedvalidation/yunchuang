package controllers

import (
	"strconv"

	"github.com/gin-gonic/gin"

	"metaclouds-backend/models"
	"metaclouds-backend/pkg/response"
	"metaclouds-backend/services"
)

type TopologyController struct {
	topologyService TopologyServiceInterface
}

func NewTopologyController(topologyService TopologyServiceInterface) *TopologyController {
	return &TopologyController{topologyService: topologyService}
}

func (c *TopologyController) GetNodeTopologies(ctx *gin.Context) {
	clusterID, _ := strconv.ParseUint(ctx.Query("cluster_id"), 10, 32)
	topologies, err := c.topologyService.ListNodeTopologies(uint(clusterID))
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, topologies)
}

func (c *TopologyController) GetNodeTopology(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	topo, err := c.topologyService.GetNodeTopology(uint(id))
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, topo)
}

func (c *TopologyController) CreateNodeTopology(ctx *gin.Context) {
	var topo models.NodeTopology
	if err := ctx.ShouldBindJSON(&topo); err != nil {
		response.Error(ctx, err)
		return
	}
	if err := c.topologyService.CreateNodeTopology(&topo); err != nil {
		response.Error(ctx, err)
		return
	}
	response.Created(ctx, topo)
}

func (c *TopologyController) UpdateNodeTopology(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	var topo models.NodeTopology
	if err := ctx.ShouldBindJSON(&topo); err != nil {
		response.Error(ctx, err)
		return
	}
	topo.ID = uint(id)
	if err := c.topologyService.UpdateNodeTopology(&topo); err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, topo)
}

func (c *TopologyController) DeleteNodeTopology(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	if err := c.topologyService.DeleteNodeTopology(uint(id)); err != nil {
		response.Error(ctx, err)
		return
	}
	response.NoContent(ctx)
}

type TopologyScoreRequest struct {
	JobID         uint     `json:"job_id" binding:"required"`
	CandidateNodes []string `json:"candidate_nodes" binding:"required"`
}

func (c *TopologyController) CalculateTopologyScore(ctx *gin.Context) {
	var req TopologyScoreRequest
	if err := ctx.ShouldBindJSON(&req); err != nil {
		response.Error(ctx, err)
		return
	}

	// 从服务获取 job（简化：直接构造 job 对象）
	job := &models.Job{ID: req.JobID}
	scores, err := c.topologyService.CalculateTopologyScore(job, req.CandidateNodes)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, scores)
}

// TopologyServiceInterface 定义拓扑服务接口
type TopologyServiceInterface interface {
	ListNodeTopologies(clusterID uint) ([]models.NodeTopology, error)
	GetNodeTopology(id uint) (*models.NodeTopology, error)
	CreateNodeTopology(topo *models.NodeTopology) error
	UpdateNodeTopology(topo *models.NodeTopology) error
	DeleteNodeTopology(id uint) error
	CalculateTopologyScore(job *models.Job, candidateNodes []string) (map[string]float64, error)
}

var _ TopologyServiceInterface = (*services.TopologyService)(nil)
