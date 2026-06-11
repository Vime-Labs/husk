package main

import (
	"fmt"
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

func jwt_verify(tokenStr string, secret string) (map[string]interface{}, error) {
	token, err := jwt.Parse(tokenStr, func(t *jwt.Token) (interface{}, error) {
		if _, ok := t.Method.(*jwt.SigningMethodHMAC); !ok {
			return nil, fmt.Errorf("husk/jwt: método de assinatura inesperado: %v", t.Header["alg"])
		}
		return []byte(secret), nil
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
