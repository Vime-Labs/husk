package main

import (
	"fmt"
	"strings"
	"time"

	"github.com/golang-jwt/jwt/v5"
)

func jwt_sign(payload map[string]interface{}, secret string) (string, error) {
	claims := jwt.MapClaims{}
	for k, v := range payload {
		claims[k] = v
	}
	if _, ok := claims["exp"]; !ok {
		claims["exp"] = time.Now().Add(24 * time.Hour).Unix()
	}
	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
	return token.SignedString([]byte(secret))
}

func jwt_sign_rs256(payload map[string]interface{}, privateKeyPem string) (string, error) {
	claims := jwt.MapClaims{}
	for k, v := range payload {
		claims[k] = v
	}
	if _, ok := claims["exp"]; !ok {
		claims["exp"] = time.Now().Add(24 * time.Hour).Unix()
	}
	key, err := jwt.ParseRSAPrivateKeyFromPEM([]byte(privateKeyPem))
	if err != nil {
		return "", fmt.Errorf("husk/jwt: chave privada RSA inválida: %v", err)
	}
	token := jwt.NewWithClaims(jwt.SigningMethodRS256, claims)
	return token.SignedString(key)
}

func jwt_verify(tokenStr string, key string) (map[string]interface{}, error) {
	token, err := jwt.Parse(tokenStr, func(t *jwt.Token) (interface{}, error) {
		alg, _ := t.Header["alg"].(string)
		switch {
		case strings.HasPrefix(alg, "HS"):
			if _, ok := t.Method.(*jwt.SigningMethodHMAC); !ok {
				return nil, fmt.Errorf("husk/jwt: método de assinatura inesperado: %v", alg)
			}
			return []byte(key), nil
		case strings.HasPrefix(alg, "RS"):
			if _, ok := t.Method.(*jwt.SigningMethodRSA); !ok {
				return nil, fmt.Errorf("husk/jwt: método de assinatura inesperado: %v", alg)
			}
			return jwt.ParseRSAPublicKeyFromPEM([]byte(key))
		default:
			return nil, fmt.Errorf("husk/jwt: algoritmo não suportado: %v", alg)
		}
	})
	if err != nil {
		return nil, err
	}
	claims, ok := token.Claims.(jwt.MapClaims)
	if !ok || !token.Valid {
		return nil, fmt.Errorf("husk/jwt: token inválido")
	}
	result := make(map[string]interface{})
	for k, v := range claims {
		result[k] = v
	}
	return result, nil
}
