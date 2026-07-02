package kernel

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
)

func TestSwitchProxyEscapesPathAndEncodesJSON(t *testing.T) {
	var gotPath string
	var gotBody struct {
		Name string `json:"name"`
	}

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		gotPath = r.URL.EscapedPath()
		if err := json.NewDecoder(r.Body).Decode(&gotBody); err != nil {
			t.Fatalf("decode body: %v", err)
		}
		w.WriteHeader(http.StatusNoContent)
	}))
	defer srv.Close()

	client := NewClient(srv.URL, "")
	if err := client.SwitchProxy("Auto/香港", `node "1"`); err != nil {
		t.Fatalf("SwitchProxy: %v", err)
	}

	if gotPath != "/proxies/Auto%2F%E9%A6%99%E6%B8%AF" {
		t.Fatalf("path = %q", gotPath)
	}
	if gotBody.Name != `node "1"` {
		t.Fatalf("body name = %q", gotBody.Name)
	}
}
