package main

import (
	"encoding/json"
	"fmt"
	"math/rand"
	"net/http"
	"os"
	"path/filepath"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/joho/godotenv"
)

const (
	publicDir        = "./public"
	garyDir          = "Gary"
	gooberDir        = "Goober"
	defaultGaryImg   = "Gary76.jpg"
	defaultGooberImg = "goober8.jpg"
	jsonDir          = "./json"
	quotesFile       = "lines.json"
	jokesFile        = "jokes.json"
)

func getFileNameFromDir(dirPath, defaultName string) string {
	files, err := os.ReadDir(dirPath)
	if err != nil || len(files) == 0 {
		return defaultName
	}
	return files[rand.Intn(len(files))].Name()
}

func getRandomLineFromFile(filePath string) (string, error) {
	fileContent, err := os.ReadFile(filePath)
	if err != nil {
		return "", fmt.Errorf("could not read file %s: %w", filePath, err)
	}

	var lines []string
	err = json.Unmarshal(fileContent, &lines)
	if err != nil {
		return "", fmt.Errorf("could not unmarshal JSON from %s: %w", filePath, err)
	}

	if len(lines) == 0 {
		return "", fmt.Errorf("no lines found in %s", filePath)
	}
	return lines[rand.Intn(len(lines))], nil
}

func serveRandomImageHandler(imageDir, defaultImage string) gin.HandlerFunc {
	return func(c *gin.Context) {
		c.Header("Cache-Control", "no-store")
		filePath := filepath.Join(publicDir, imageDir, getFileNameFromDir(filepath.Join(publicDir, imageDir), defaultImage))
		c.File(filePath)
	}
}

func serveImageURLHandler(baseURL, imageDir, defaultImage string) gin.HandlerFunc {
	return func(c *gin.Context) {
		imageName := getFileNameFromDir(filepath.Join(publicDir, imageDir), defaultImage)
		cleanBaseURL := baseURL
		if len(cleanBaseURL) > 0 && cleanBaseURL[len(cleanBaseURL)-1] == '/' {
			cleanBaseURL = cleanBaseURL[:len(cleanBaseURL)-1]
		}
		url := fmt.Sprintf("%s/%s", cleanBaseURL, imageName)
		c.JSON(http.StatusOK, gin.H{"url": url})
	}
}

func serveRandomLineHandler(filePath string) gin.HandlerFunc {
	return func(c *gin.Context) {
		line, err := getRandomLineFromFile(filePath)
		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
			return
		}
		c.JSON(http.StatusOK, gin.H{filepath.Base(filePath): line})
	}
}

func main() {
	_ = godotenv.Load()

	rand.Seed(time.Now().UnixNano())

	gin.SetMode(gin.ReleaseMode)
	r := gin.Default()

	r.Static("/Gary", filepath.Join(publicDir, garyDir))
	r.Static("/Goober", filepath.Join(publicDir, gooberDir))

	imageRoutes := r.Group("/")
	{
		imageRoutes.GET("/gary/image", serveRandomImageHandler(garyDir, defaultGaryImg))
		imageRoutes.GET("/goober/image", serveRandomImageHandler(gooberDir, defaultGooberImg))
	}

	apiRoutes := r.Group("/")
	{
		garyBaseURL := os.Getenv("GARYURL")
		gooberBaseURL := os.Getenv("GOOBERURL")

		apiRoutes.GET("/gary", serveImageURLHandler(garyBaseURL, garyDir, defaultGaryImg))
		apiRoutes.GET("/goober", serveImageURLHandler(gooberBaseURL, gooberDir, defaultGooberImg))
		apiRoutes.GET("/quote", serveRandomLineHandler(filepath.Join(jsonDir, quotesFile)))
		apiRoutes.GET("/joke", serveRandomLineHandler(filepath.Join(jsonDir, jokesFile)))
	}

	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}
	if err := r.Run(":" + port); err != nil {
		fmt.Printf("Failed to start the server: %v\n", err)
	}
}
