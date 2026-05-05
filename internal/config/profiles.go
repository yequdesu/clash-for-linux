package config

import (
	"fmt"
	"os"
	"strconv"

	"gopkg.in/yaml.v3"
)

type Profile struct {
	ID       int    `yaml:"id"`
	Path     string `yaml:"path"`
	URL      string `yaml:"url"`
	Name     string `yaml:"name,omitempty"`
	Updated  string `yaml:"updated,omitempty"`
	Interval string `yaml:"interval,omitempty"`
}

type ProfilesMeta struct {
	Use      int       `yaml:"use"`
	Profiles []Profile `yaml:"profiles"`
}

func LoadProfiles(path string) (*ProfilesMeta, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return &ProfilesMeta{}, nil
	}
	var m ProfilesMeta
	if err := yaml.Unmarshal(data, &m); err != nil {
		return nil, fmt.Errorf("parse profiles: %w", err)
	}
	return &m, nil
}

func SaveProfiles(path string, m *ProfilesMeta) error {
	data, err := yaml.Marshal(m)
	if err != nil {
		return err
	}
	return os.WriteFile(path, data, 0644)
}

func (m *ProfilesMeta) NextID() int {
	maxID := 0
	for _, p := range m.Profiles {
		if p.ID > maxID {
			maxID = p.ID
		}
	}
	return maxID + 1
}

func (m *ProfilesMeta) FindByID(id int) *Profile {
	for i := range m.Profiles {
		if m.Profiles[i].ID == id {
			return &m.Profiles[i]
		}
	}
	return nil
}

func (m *ProfilesMeta) FindByURL(url string) *Profile {
	for i := range m.Profiles {
		if m.Profiles[i].URL == url {
			return &m.Profiles[i]
		}
	}
	return nil
}

func (m *ProfilesMeta) RemoveByID(id int) {
	for i := range m.Profiles {
		if m.Profiles[i].ID == id {
			m.Profiles = append(m.Profiles[:i], m.Profiles[i+1:]...)
			return
		}
	}
}

func (m *ProfilesMeta) GetCurrent() *Profile {
	for i := range m.Profiles {
		if m.Profiles[i].ID == m.Use {
			return &m.Profiles[i]
		}
	}
	return nil
}

func ParseInt(s string) (int, error) {
	return strconv.Atoi(s)
}
