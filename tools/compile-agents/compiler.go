package main

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"text/template"
)

// TemplateData holds all values injected into the agent template.
type TemplateData struct {
	MOSCode         string
	MOSName         string
	Description     string
	Model           string
	RankCode        string
	RankName        string
	Category        string
	AuthorityLevel  int
	DecisionPosture string
	MOSDoctrine     string
	SitrepProtocol  string
	CasrepProtocol  string
	FragoProtocol   string
	AARProtocol     string
}

// Protocols holds the content of the four shared protocol files.
type Protocols struct {
	SitRep string
	CasRep string
	FRAGO  string
	AAR    string
}

// loadProtocols reads the four protocol markdown files from doctrineDir/protocols
// and returns a populated Protocols value.
func loadProtocols(doctrineDir string) (Protocols, error) {
	dir := filepath.Join(doctrineDir, "protocols")

	read := func(name string) (string, error) {
		data, err := os.ReadFile(filepath.Join(dir, name))
		if err != nil {
			return "", fmt.Errorf("read protocol %s: %w", name, err)
		}
		return strings.TrimSpace(string(data)), nil
	}

	sitrep, err := read("sitrep.md")
	if err != nil {
		return Protocols{}, err
	}
	casrep, err := read("casrep.md")
	if err != nil {
		return Protocols{}, err
	}
	frago, err := read("frago.md")
	if err != nil {
		return Protocols{}, err
	}
	aar, err := read("aar.md")
	if err != nil {
		return Protocols{}, err
	}

	return Protocols{
		SitRep: sitrep,
		CasRep: casrep,
		FRAGO:  frago,
		AAR:    aar,
	}, nil
}

// extractDecisionPosture finds the "## Decision posture" section in rankContent
// and returns the body text (all lines until the next heading or EOF).
// The heading line itself is not included in the return value.
func extractDecisionPosture(rankContent string) string {
	lines := strings.Split(rankContent, "\n")

	inSection := false
	var body []string

	for _, line := range lines {
		if inSection {
			// Stop at any new heading (## or #)
			if strings.HasPrefix(line, "#") {
				break
			}
			body = append(body, line)
		} else {
			// Match heading case-insensitively on "decision posture"
			trimmed := strings.TrimLeft(line, "#")
			trimmed = strings.TrimSpace(trimmed)
			if strings.EqualFold(trimmed, "decision posture") && strings.HasPrefix(line, "##") {
				inSection = true
			}
		}
	}

	// Strip leading and trailing blank lines from the body
	result := strings.Join(body, "\n")
	result = strings.TrimSpace(result)
	return result
}

// readFile reads a file and returns its content as a string.
func readFile(path string) (string, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return "", fmt.Errorf("read %s: %w", path, err)
	}
	return string(data), nil
}

// compileAgentToBytes compiles a single agent using the entry metadata,
// a pre-parsed template, pre-loaded protocols, and the doctrine directory.
// It returns the rendered template bytes.
func compileAgentToBytes(entry MOSEntry, doctrineDir string, tmpl *template.Template, protocols Protocols) ([]byte, error) {
	mosDoctrine, err := readFile(filepath.Join(doctrineDir, "mos", entry.DoctrineFile))
	if err != nil {
		return nil, fmt.Errorf("MOS doctrine for %s: %w", entry.Code, err)
	}

	rankContent, err := readFile(filepath.Join(doctrineDir, "ranks", entry.RankFile))
	if err != nil {
		return nil, fmt.Errorf("rank file for %s: %w", entry.Code, err)
	}

	data := TemplateData{
		MOSCode:         entry.Code,
		MOSName:         entry.Name,
		Description:     entry.Description,
		Model:           entry.Model,
		RankCode:        entry.DefaultRank,
		RankName:        entry.RankName,
		Category:        entry.Category,
		AuthorityLevel:  entry.AuthLevel,
		DecisionPosture: extractDecisionPosture(rankContent),
		MOSDoctrine:     strings.TrimSpace(mosDoctrine),
		SitrepProtocol:  protocols.SitRep,
		CasrepProtocol:  protocols.CasRep,
		FragoProtocol:   protocols.FRAGO,
		AARProtocol:     protocols.AAR,
	}

	var buf bytes.Buffer
	if err := tmpl.Execute(&buf, data); err != nil {
		return nil, fmt.Errorf("execute template for %s: %w", entry.Code, err)
	}

	return buf.Bytes(), nil
}

// compileAll iterates the MOS table, compiles each agent, and writes only files
// whose content has changed. Progress lines are written to w. Returns the count
// of files actually written.
func compileAll(table []MOSEntry, doctrineDir, templatePath, outDir string, w io.Writer) (int, error) {
	tmplContent, err := readFile(templatePath)
	if err != nil {
		return 0, fmt.Errorf("template: %w", err)
	}
	tmpl, err := template.New("agent").Parse(tmplContent)
	if err != nil {
		return 0, fmt.Errorf("parse template: %w", err)
	}

	protocols, err := loadProtocols(doctrineDir)
	if err != nil {
		return 0, err
	}

	written := 0

	for _, entry := range table {
		rendered, err := compileAgentToBytes(entry, doctrineDir, tmpl, protocols)
		if err != nil {
			return written, fmt.Errorf("compile %s: %w", entry.Code, err)
		}

		outPath := filepath.Join(outDir, entry.AgentFile)

		existing, readErr := os.ReadFile(outPath)
		if readErr == nil && bytes.Equal(existing, rendered) {
			fmt.Fprintf(w, "unchanged: %s\n", entry.AgentFile)
			continue
		}

		if err := os.WriteFile(outPath, rendered, 0644); err != nil {
			return written, fmt.Errorf("write %s: %w", entry.AgentFile, err)
		}

		fmt.Fprintf(w, "wrote: %s\n", entry.AgentFile)
		written++
	}

	return written, nil
}
