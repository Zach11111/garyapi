package main

import (
	"github.com/gin-gonic/gin"
	"math/rand"
	"os"
	"path/filepath"
	"time"
	"net/http"
	"encoding/json"
	"fmt"
	"github.com/joho/godotenv"
)

func randomGary() string {
	files, err := os.ReadDir("./public/Gary")
	if err != nil {
		println("Error reading directory:", err)
		return "Gary76.jpg"
	}
	return files[rand.Intn(len(files))].Name()
}

func randomGoober() string {
	files, err := os.ReadDir("./public/Goober")
	if err != nil {
		println("Error reading directory:", err)
		return "goober8.jpg"
	}
	return files[rand.Intn(len(files))].Name()
}

func getRandomLineFromFile(filePath string) (string, error) {
	file, err := os.ReadFile(filePath)
	if err != nil {
		return "", err
	}

	var quotes []string
	err = json.Unmarshal(file, &quotes)
	if err != nil {
		return "", err
	}

	if len(quotes) == 0 {
		return "", fmt.Errorf("no quotes found")
	}
	rand.Seed(time.Now().UnixNano())
	randomQuote := quotes[rand.Intn(len(quotes))]

	return randomQuote, nil
}

func main() {
	env := godotenv.Load()
	if env != nil {
		println("Error loading .env file:", env)
	}


	rand.Seed(time.Now().UnixNano())
	gin.SetMode(gin.ReleaseMode)
	r := gin.Default()


	r.GET("/gary/image", func(c *gin.Context) {
		filePath := filepath.Join("public", "Gary", randomGary())
		c.Header("Cache-Control", "no-store")
		c.File(filePath)
	})

	r.GET("/goober/image", func(c *gin.Context) {
		filePath := filepath.Join("public", "Goober", randomGoober())
		c.Header("Cache-Control", "no-store")
		c.File(filePath)
	})

	r.GET("/gary", func(c *gin.Context) {
		url := "https://cdn.garybot.dev/" + randomGary()
		c.JSON(http.StatusOK, gin.H{"url": url})
	})
	
	r.GET("/goober", func(c *gin.Context) {
		url := "https://goober.garybot.dev/" + randomGoober()
		c.JSON(http.StatusOK, gin.H{"url": url})
	})

	r.GET("/quote", func(c *gin.Context) {
		quote, err := getRandomLineFromFile("./json/quotes.json")
		if err != nil {
			c.JSON(500, gin.H{"error": err.Error()})
			return
		}
		c.JSON(200, gin.H{"quote": quote})
	})

	r.GET("/joke", func(c *gin.Context) {
		joke, err := getRandomLineFromFile("./json/jokes.json")
		if err != nil {
			c.JSON(500, gin.H{"error": err.Error()})
			return
		}
		c.JSON(200, gin.H{"joke": joke})
	})

	r.Static("/Gary", "./public/Gary")
	r.Static("/Goober", "./public/Goober")
	r.Run(":" + os.Getenv("PORT"))
}
