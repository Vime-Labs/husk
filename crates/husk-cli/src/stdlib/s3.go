package main

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"time"

	"github.com/minio/minio-go/v7"
	"github.com/minio/minio-go/v7/pkg/credentials"
)

func newMinioClient(endpoint, accessKey, secretKey, region string) (*minio.Client, error) {
	if region == "" {
		region = "us-east-1"
	}
	return minio.New(endpoint, &minio.Options{
		Creds:  credentials.NewStaticV4(accessKey, secretKey, ""),
		Secure: true,
		Region: region,
	})
}

func s3_get_object(endpoint, accessKey, secretKey, region, bucket, key string) ([]byte, error) {
	client, err := newMinioClient(endpoint, accessKey, secretKey, region)
	if err != nil {
		return nil, fmt.Errorf("husk/s3: erro ao criar cliente: %v", err)
	}
	obj, err := client.GetObject(context.Background(), bucket, key, minio.GetObjectOptions{})
	if err != nil {
		return nil, fmt.Errorf("husk/s3: erro ao obter objeto: %v", err)
	}
	defer obj.Close()
	data, err := io.ReadAll(obj)
	if err != nil {
		return nil, fmt.Errorf("husk/s3: erro ao ler objeto: %v", err)
	}
	return data, nil
}

func s3_put_object(endpoint, accessKey, secretKey, region, bucket, key string, data []byte) (string, error) {
	client, err := newMinioClient(endpoint, accessKey, secretKey, region)
	if err != nil {
		return "", fmt.Errorf("husk/s3: erro ao criar cliente: %v", err)
	}
	info, err := client.PutObject(context.Background(), bucket, key, bytes.NewReader(data), int64(len(data)), minio.PutObjectOptions{})
	if err != nil {
		return "", fmt.Errorf("husk/s3: erro ao enviar objeto: %v", err)
	}
	return info.ETag, nil
}

func s3_delete_object(endpoint, accessKey, secretKey, region, bucket, key string) error {
	client, err := newMinioClient(endpoint, accessKey, secretKey, region)
	if err != nil {
		return fmt.Errorf("husk/s3: erro ao criar cliente: %v", err)
	}
	err = client.RemoveObject(context.Background(), bucket, key, minio.RemoveObjectOptions{})
	if err != nil {
		return fmt.Errorf("husk/s3: erro ao remover objeto: %v", err)
	}
	return nil
}

func s3_list_objects(endpoint, accessKey, secretKey, region, bucket, prefix string) ([]map[string]interface{}, error) {
	client, err := newMinioClient(endpoint, accessKey, secretKey, region)
	if err != nil {
		return nil, fmt.Errorf("husk/s3: erro ao criar cliente: %v", err)
	}
	opts := minio.ListObjectsOptions{
		Prefix:    prefix,
		Recursive: true,
	}
	var result []map[string]interface{}
	for obj := range client.ListObjects(context.Background(), bucket, opts) {
		if obj.Err != nil {
			return nil, fmt.Errorf("husk/s3: erro ao listar objetos: %v", obj.Err)
		}
		result = append(result, map[string]interface{}{
			"key":           obj.Key,
			"size":          obj.Size,
			"etag":          obj.ETag,
			"last_modified": obj.LastModified.Unix(),
		})
	}
	return result, nil
}

func s3_presigned_url(endpoint, accessKey, secretKey, region, bucket, key string, expiry int64) (string, error) {
	client, err := newMinioClient(endpoint, accessKey, secretKey, region)
	if err != nil {
		return "", fmt.Errorf("husk/s3: erro ao criar cliente: %v", err)
	}
	url, err := client.PresignedGetObject(context.Background(), bucket, key, time.Duration(expiry)*time.Second, nil)
	if err != nil {
		return "", fmt.Errorf("husk/s3: erro ao gerar URL: %v", err)
	}
	return url.String(), nil
}

func s3_presigned_put_url(endpoint, accessKey, secretKey, region, bucket, key string, expiry int64) (string, error) {
	client, err := newMinioClient(endpoint, accessKey, secretKey, region)
	if err != nil {
		return "", fmt.Errorf("husk/s3: erro ao criar cliente: %v", err)
	}
	url, err := client.PresignedPutObject(context.Background(), bucket, key, time.Duration(expiry)*time.Second)
	if err != nil {
		return "", fmt.Errorf("husk/s3: erro ao gerar URL de upload: %v", err)
	}
	return url.String(), nil
}
