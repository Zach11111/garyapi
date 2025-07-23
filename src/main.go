package main

import (
	"encoding/json"
	"fmt"
	"math/rand"
	"net/http"
	"os"
	"path/filepath"
	"regexp"
	"runtime"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/joho/godotenv"
)

const (
	defaultGaryImg   = "Gary76.jpg"
	defaultGooberImg = "goober8.jpg"
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

func extractNumberFromFilename(filename string) int {
	re := regexp.MustCompile(`\d+`)
	match := re.FindString(filename)
	if match == "" {
		return 0
	}
	var number int
	fmt.Sscanf(match, "%d", &number)
	return number
}

func serveRandomImageHandler(imageDir, defaultImage string) gin.HandlerFunc {
	return func(c *gin.Context) {
		c.Header("Cache-Control", "no-store")
		filePath := filepath.Join(imageDir, getFileNameFromDir(imageDir, defaultImage))
		c.File(filePath)
	}
}

func serveImageURLHandler(baseURL, imageDir, defaultImage string) gin.HandlerFunc {
	return func(c *gin.Context) {
		imageName := getFileNameFromDir(imageDir, defaultImage)
		number := extractNumberFromFilename(imageName)

		cleanBaseURL := baseURL
		if len(cleanBaseURL) > 0 && cleanBaseURL[len(cleanBaseURL)-1] == '/' {
			cleanBaseURL = cleanBaseURL[:len(cleanBaseURL)-1]
		}
		url := fmt.Sprintf("%s/%s", cleanBaseURL, imageName)

		c.JSON(http.StatusOK, gin.H{
			"url":    url,
			"number": number,
		})
	}
}

func serveRandomLineHandler(filePath string) gin.HandlerFunc {
	return func(c *gin.Context) {
		line, err := getRandomLineFromFile(filePath)
		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
			return
		}

		var key string
		switch filepath.Base(filePath) {
		case filepath.Base(os.Getenv("QUOTES_FILE")):
			key = "quote"
		case filepath.Base(os.Getenv("JOKES_FILE")):
			key = "joke"
		default:
			key = "line"
		}

		c.JSON(http.StatusOK, gin.H{key: line})
	}
}

func countFilesInDir(dirPath string) int {
	files, err := os.ReadDir(dirPath)
	if err != nil {
		fmt.Printf("ERROR reading dir %s: %v\n", dirPath, err)
		return 0
	}
	return len(files)
}


func main() {
	_ = godotenv.Load()

	runtime.GOMAXPROCS(runtime.NumCPU())
	rand.Seed(time.Now().UnixNano())
	gin.SetMode(gin.ReleaseMode)
	r := gin.Default()

	garyDir := os.Getenv("GARY_DIR")
	gooberDir := os.Getenv("GOOBER_DIR")
	quotesPath := os.Getenv("QUOTES_FILE")
	jokesPath := os.Getenv("JOKES_FILE")

	r.Static("/Gary", garyDir)
	r.Static("/Goober", gooberDir)

	imageRoutes := r.Group("/")
	{
		imageRoutes.GET("/gary/image/*path", serveRandomImageHandler(garyDir, defaultGaryImg))
		imageRoutes.GET("/goober/image/*path", serveRandomImageHandler(gooberDir, defaultGooberImg))
	}

	apiRoutes := r.Group("/")
	{
		garyBaseURL := os.Getenv("GARYURL")
		gooberBaseURL := os.Getenv("GOOBERURL")

		apiRoutes.GET("/gary", serveImageURLHandler(garyBaseURL, garyDir, defaultGaryImg))
		apiRoutes.GET("/goober", serveImageURLHandler(gooberBaseURL, gooberDir, defaultGooberImg))
		apiRoutes.GET("/quote", serveRandomLineHandler(quotesPath))
		apiRoutes.GET("/joke", serveRandomLineHandler(jokesPath))

		apiRoutes.GET("/gary/count", func(c *gin.Context) {
			c.JSON(http.StatusOK, gin.H{"count": countFilesInDir(garyDir)})
		})
		apiRoutes.GET("/goober/count", func(c *gin.Context) {
			c.JSON(http.StatusOK, gin.H{"count": countFilesInDir(gooberDir)})
		})
	}

	indexFile := os.Getenv("INDEX_FILE")
	if indexFile != "" {
		r.GET("/", func(c *gin.Context) {
			c.Header("Cache-Control", "no-store")
			c.File(indexFile)
		})
	}

	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}
	if err := r.Run(":" + port); err != nil {
		fmt.Printf("Failed to start the server: %v\n", err)
	}
}
