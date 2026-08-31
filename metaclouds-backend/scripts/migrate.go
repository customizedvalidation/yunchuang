package main

import (
	"database/sql"
	"fmt"
	"io/ioutil"
	"os"

	_ "github.com/mattn/go-sqlite3"
)

func main() {
	if len(os.Args) < 3 {
		fmt.Println("Usage: go run migrate.go <database_file> <sql_file>")
		os.Exit(1)
	}

	dbFile := os.Args[1]
	sqlFile := os.Args[2]

	fmt.Printf("Migrating database: %s\n", dbFile)
	fmt.Printf("Using migration file: %s\n", sqlFile)

	db, err := sql.Open("sqlite3", dbFile)
	if err != nil {
		fmt.Printf("Failed to open database: %v\n", err)
		os.Exit(1)
	}
	defer db.Close()

	sqlContent, err := ioutil.ReadFile(sqlFile)
	if err != nil {
		fmt.Printf("Failed to read SQL file: %v\n", err)
		os.Exit(1)
	}

	_, err = db.Exec(string(sqlContent))
	if err != nil {
		fmt.Printf("Migration failed: %v\n", err)
		os.Exit(1)
	}

	fmt.Println("Migration completed successfully!")

	rows, err := db.Query("SELECT name FROM sqlite_master WHERE type='index' AND name LIKE 'idx_%'")
	if err != nil {
		fmt.Printf("Failed to verify indexes: %v\n", err)
		return
	}
	defer rows.Close()

	fmt.Println("\nCreated indexes:")
	for rows.Next() {
		var name string
		if err := rows.Scan(&name); err != nil {
			fmt.Printf("Failed to scan index name: %v\n", err)
			return
		}
		fmt.Printf("  - %s\n", name)
	}
}
