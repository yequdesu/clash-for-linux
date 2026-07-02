package config

import (
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"

	"gopkg.in/yaml.v3"
)

func UpdateYAML(path string, updates map[string]any) error {
	var doc yaml.Node
	if data, err := os.ReadFile(path); err == nil && len(strings.TrimSpace(string(data))) > 0 {
		if err := yaml.Unmarshal(data, &doc); err != nil {
			return fmt.Errorf("parse yaml: %w", err)
		}
	} else if err != nil && !os.IsNotExist(err) {
		return err
	}

	root := ensureDocumentMapping(&doc)
	for path, value := range updates {
		if err := setYAMLPath(root, strings.Split(path, "."), yamlValueNode(value)); err != nil {
			return err
		}
	}

	out, err := yaml.Marshal(&doc)
	if err != nil {
		return fmt.Errorf("marshal yaml: %w", err)
	}
	if err := os.MkdirAll(filepath.Dir(path), 0755); err != nil {
		return err
	}
	return AtomicWriteFile(path, out, 0644)
}

func ensureDocumentMapping(doc *yaml.Node) *yaml.Node {
	if doc.Kind == 0 {
		doc.Kind = yaml.DocumentNode
	}
	if doc.Kind != yaml.DocumentNode {
		original := *doc
		*doc = yaml.Node{Kind: yaml.DocumentNode, Content: []*yaml.Node{&original}}
	}
	if len(doc.Content) == 0 || doc.Content[0].Kind != yaml.MappingNode {
		doc.Content = []*yaml.Node{{Kind: yaml.MappingNode, Tag: "!!map"}}
	}
	return doc.Content[0]
}

func setYAMLPath(root *yaml.Node, path []string, value *yaml.Node) error {
	if len(path) == 0 || strings.TrimSpace(path[0]) == "" {
		return fmt.Errorf("empty yaml path")
	}
	if root.Kind != yaml.MappingNode {
		return fmt.Errorf("yaml path %q is not a mapping", strings.Join(path, "."))
	}

	key := path[0]
	for i := 0; i+1 < len(root.Content); i += 2 {
		if root.Content[i].Value != key {
			continue
		}
		if len(path) == 1 {
			root.Content[i+1] = value
			return nil
		}
		if root.Content[i+1].Kind != yaml.MappingNode {
			root.Content[i+1] = &yaml.Node{Kind: yaml.MappingNode, Tag: "!!map"}
		}
		return setYAMLPath(root.Content[i+1], path[1:], value)
	}

	root.Content = append(root.Content, &yaml.Node{Kind: yaml.ScalarNode, Tag: "!!str", Value: key})
	if len(path) == 1 {
		root.Content = append(root.Content, value)
		return nil
	}
	child := &yaml.Node{Kind: yaml.MappingNode, Tag: "!!map"}
	root.Content = append(root.Content, child)
	return setYAMLPath(child, path[1:], value)
}

func yamlValueNode(value any) *yaml.Node {
	switch v := value.(type) {
	case bool:
		return &yaml.Node{Kind: yaml.ScalarNode, Tag: "!!bool", Value: strconv.FormatBool(v)}
	case int:
		return &yaml.Node{Kind: yaml.ScalarNode, Tag: "!!int", Value: strconv.Itoa(v)}
	case []string:
		node := &yaml.Node{Kind: yaml.SequenceNode, Tag: "!!seq"}
		for _, item := range v {
			node.Content = append(node.Content, &yaml.Node{Kind: yaml.ScalarNode, Tag: "!!str", Value: item})
		}
		return node
	default:
		return &yaml.Node{Kind: yaml.ScalarNode, Tag: "!!str", Value: fmt.Sprint(v)}
	}
}
