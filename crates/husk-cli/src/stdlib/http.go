package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"mime/multipart"
	"net/http"
	"os"
	"path/filepath"
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
	headers := make(map[string]string)
	query := make(map[string]string)
	var timeout time.Duration
	var multipartFields map[string]interface{}

	if len(options) > 0 {
		opts := options[0]
		if h, ok := opts["headers"]; ok {
			if m, ok := h.(map[string]interface{}); ok {
				for k, v := range m {
					headers[k] = fmt.Sprintf("%v", v)
				}
			}
		}
		if q, ok := opts["query"]; ok {
			if m, ok := q.(map[string]interface{}); ok {
				for k, v := range m {
					query[k] = fmt.Sprintf("%v", v)
				}
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
		if mp, ok := opts["multipart"]; ok {
			if m, ok := mp.(map[string]interface{}); ok {
				multipartFields = m
			}
		}
	}

	var reqBody io.Reader
	if multipartFields != nil {
		var buf bytes.Buffer
		w := multipart.NewWriter(&buf)
		for key, val := range multipartFields {
			switch v := val.(type) {
			case string:
				w.WriteField(key, v)
			case map[string]interface{}:
				fpath, _ := v["path"].(string)
				filename, _ := v["filename"].(string)
				if fpath == "" {
					continue
				}
				if filename == "" {
					filename = filepath.Base(fpath)
				}
				fw, err := w.CreateFormFile(key, filename)
				if err != nil {
					w.Close()
					return nil, fmt.Errorf("husk/http: erro ao criar campo file: %v", err)
				}
				f, err := os.Open(fpath)
				if err != nil {
					w.Close()
					return nil, fmt.Errorf("husk/http: erro ao abrir '%s': %v", fpath, err)
				}
				_, err = io.Copy(fw, f)
				f.Close()
				if err != nil {
					w.Close()
					return nil, fmt.Errorf("husk/http: erro ao ler '%s': %v", fpath, err)
				}
			}
		}
		w.Close()
		reqBody = &buf
		headers["Content-Type"] = w.FormDataContentType()
	} else if body != nil {
		switch v := body.(type) {
		case string:
			reqBody = strings.NewReader(v)
		default:
			b, err := json.Marshal(v)
			if err != nil {
				return nil, fmt.Errorf("husk/http: erro ao serializar body: %v", err)
			}
			reqBody = bytes.NewReader(b)
			if _, ok := headers["Content-Type"]; !ok {
				headers["Content-Type"] = "application/json"
			}
		}
	}

	req, err := http.NewRequest(method, url, reqBody)
	if err != nil {
		return nil, fmt.Errorf("husk/http: erro ao criar requisição: %v", err)
	}

	for k, v := range headers {
		req.Header.Set(k, v)
	}

	if len(query) > 0 {
		q := req.URL.Query()
		for k, v := range query {
			q.Set(k, v)
		}
		req.URL.RawQuery = q.Encode()
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
