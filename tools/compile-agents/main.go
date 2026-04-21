package main

import (
	"fmt"
	"os"
	"path/filepath"
)

func main() {
	homeDir, err := os.UserHomeDir()
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: cannot determine home directory: %v\n", err)
		os.Exit(1)
	}

	doctrineDir := filepath.Join(homeDir, "Projects", "rational-reserve", "doctrine")
	templatePath := filepath.Join(homeDir, "Projects", "rational-reserve", "adapters", "agent-template.md")
	outDir := filepath.Join(homeDir, ".rational-reserve", "agents")

	if v := os.Getenv("RR_DOCTRINE_DIR"); v != "" {
		doctrineDir = v
	}
	if v := os.Getenv("RR_AGENT_TEMPLATE"); v != "" {
		templatePath = v
	}
	if v := os.Getenv("RR_AGENTS_DIR"); v != "" {
		outDir = v
	}

	// Validate doctrine directory exists.
	if info, err := os.Stat(doctrineDir); err != nil || !info.IsDir() {
		fmt.Fprintf(os.Stderr, "error: doctrine directory not found: %s\n", doctrineDir)
		os.Exit(1)
	}

	// Validate template file exists.
	if _, err := os.Stat(templatePath); err != nil {
		fmt.Fprintf(os.Stderr, "error: agent template not found: %s\n", templatePath)
		os.Exit(1)
	}

	// Create output directory if it does not exist.
	if err := os.MkdirAll(outDir, 0755); err != nil {
		fmt.Fprintf(os.Stderr, "error: cannot create output directory %s: %v\n", outDir, err)
		os.Exit(1)
	}

	fmt.Printf("rr-compile-agents\n")
	fmt.Printf("  doctrine : %s\n", doctrineDir)
	fmt.Printf("  template : %s\n", templatePath)
	fmt.Printf("  output   : %s\n", outDir)
	fmt.Printf("  agents   : %d\n\n", len(MOSTable))

	written, err := compileAll(MOSTable, doctrineDir, templatePath, outDir, os.Stdout)
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("\nsummary: %d/%d files written\n", written, len(MOSTable))
}
