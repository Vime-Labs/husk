package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"
)

type httpResponse struct {
	Status  int               `json:"status"`
	Body    string            `json:"body"`
	Headers map[string]string `json:"headers"`
}

func http_get(url string, options ...map[string]interface{}) (*httpResponse, error) {
	return http_request("GET", url, nil, options...)
}

func http_post(url string, body interface{}, options ...map[string]interface{}) (*httpResponse, error) {
	return http_request("POST", url, body, options...)
}

func http_put(url string, body interface{}, options ...map[string]interface{}) (*httpResponse, error) {
	return http_request("PUT", url, body, options...)
}

func http_patch(url string, body interface{}, options ...map[string]interface{}) (*httpResponse, error) {
	return http_request("PATCH", url, body, options...)
}

func http_delete(url string, options ...map[string]interface{}) (*httpResponse, error) {
	return http_request("DELETE", url, nil, options...)
}

func http_request(method string, url string, body interface{}, options ...map[string]interface{}) (*httpResponse, error) {
	var reqBody io.Reader
	if body != nil {
		switch v := body.(type) {
		case string:
			reqBody = strings.NewReader(v)
		case map[string]interface{}, []interface{}:
			b, err := json.Marshal(v)
			if err != nil {
				return nil, fmt.Errorf("husk/http: erro ao serializar body: %v", err)
			}
			reqBody = bytes.NewReader(b)
		default:
			b, err := json.Marshal(v)
			if err != nil {
				return nil, fmt.Errorf("husk/http: erro ao serializar body: %v", err)
			}
			reqBody = bytes.NewReader(b)
		}
	}

	req, err := http.NewRequest(method, url, reqBody)
	if err != nil {
		return nil, fmt.Errorf("husk/http: erro ao criar requisição: %v", err)
	}

	var timeout time.Duration
	if len(options) > 0 {
		opts := options[0]
		if headers, ok := opts["headers"]; ok {
			if hMap, ok := headers.(map[string]interface{}); ok {
				for k, v := range hMap {
					req.Header.Set(k, fmt.Sprintf("%v", v))
				}
			}
		}
		if qs, ok := opts["query"]; ok {
			if qMap, ok := qs.(map[string]interface{}); ok {
				q := req.URL.Query()
				for k, v := range qMap {
					q.Set(k, fmt.Sprintf("%v", v))
				}
				req.URL.RawQuery = q.Encode()
			}
		}
		if t, ok := opts["timeout"]; ok {
			switch v := t.(type) {
			case float64:
				timeout = time.Duration(v) * time.Second
			case int64:
				timeout = time.Duration(v) * time.Second
			}
		}
	}

	client := &http.Client{Timeout: timeout}
	resp, err := client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("husk/http: requisição falhou: %v", err)
	}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("husk/http: erro ao ler resposta: %v", err)
	}

	respHeaders := make(map[string]string)
	for k := range resp.Header {
		respHeaders[k] = resp.Header.Get(k)
	}

	return &httpResponse{
		Status:  resp.StatusCode,
		Body:    string(respBody),
		Headers: respHeaders,
	}, nil
}
