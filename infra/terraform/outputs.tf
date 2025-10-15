##############################################################################
# Outputs for the Sigil AWS reference module.
##############################################################################

output "alb_dns_name" {
  description = "Public DNS name of the application load balancer."
  value       = aws_lb.this.dns_name
}

output "server_ecr_repository_url" {
  description = "Push the server image here."
  value       = aws_ecr_repository.server.repository_url
}

output "web_ecr_repository_url" {
  description = "Push the web image here."
  value       = aws_ecr_repository.web.repository_url
}

output "ecs_cluster_name" {
  description = "Name of the ECS cluster."
  value       = aws_ecs_cluster.this.name
}

output "ecs_service_name" {
  description = "Name of the ECS service running the server."
  value       = aws_ecs_service.server.name
}

output "database_endpoint" {
  description = "RDS Postgres connection endpoint (host:port)."
  value       = aws_db_instance.this.endpoint
}

output "database_address" {
  description = "RDS Postgres hostname."
  value       = aws_db_instance.this.address
}

output "vpc_id" {
  description = "Identifier of the created VPC."
  value       = aws_vpc.this.id
}
