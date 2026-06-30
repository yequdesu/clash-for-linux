package helpers

import (
	"net/http"
	"net/http/httptest"
	"testing"
)

// MockServer 创建一个 mock HTTP 服务器
func MockServer(t *testing.T, handler http.HandlerFunc) *httptest.Server {
	t.Helper()
	server := httptest.NewServer(handler)
	t.Cleanup(func() {
		server.Close()
	})
	return server
}

// MockJSONResponse 创建一个返回 JSON 的 mock 处理器
func MockJSONResponse(statusCode int, jsonBody string) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(statusCode)
		w.Write([]byte(jsonBody))
	}
}
