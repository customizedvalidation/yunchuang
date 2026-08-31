package controllers

import (
	"strconv"

	"github.com/gin-gonic/gin"

	"metaclouds-backend/pkg/response"
	"metaclouds-backend/services"
)

type ClusterController struct {
	clusterService *services.ClusterService
}

func NewClusterController(clusterService *services.ClusterService) *ClusterController {
	return &ClusterController{
		clusterService: clusterService,
	}
}

func (c *ClusterController) GetClusters(ctx *gin.Context) {
	clusters, err := c.clusterService.GetClusters()
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, clusters)
}

func (c *ClusterController) GetCluster(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	cluster, err := c.clusterService.GetCluster(uint(id))
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Success(ctx, cluster)
}

func (c *ClusterController) CreateCluster(ctx *gin.Context) {
	var req services.CreateClusterRequest
	if err := ctx.ShouldBindJSON(&req); err != nil {
		response.Error(ctx, err)
		return
	}

	cluster, err := c.clusterService.CreateCluster(req)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Created(ctx, cluster)
}

func (c *ClusterController) UpdateCluster(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	var req services.UpdateClusterRequest
	if err := ctx.ShouldBindJSON(&req); err != nil {
		response.Error(ctx, err)
		return
	}

	cluster, err := c.clusterService.UpdateCluster(uint(id), req)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Success(ctx, cluster)
}

func (c *ClusterController) DeleteCluster(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	if err := c.clusterService.DeleteCluster(uint(id)); err != nil {
		response.Error(ctx, err)
		return
	}

	response.NoContent(ctx)
}