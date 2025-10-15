##############################################################################
# Input variables for the Sigil AWS reference module.
##############################################################################

variable "project" {
  description = "Short project slug used to name resources."
  type        = string
  default     = "sigil"
}

variable "environment" {
  description = "Deployment environment (e.g. dev, staging, prod)."
  type        = string
  default     = "dev"
}

variable "aws_region" {
  description = "AWS region to deploy into."
  type        = string
  default     = "us-east-1"
}

# --- Networking -------------------------------------------------------------

variable "vpc_cidr" {
  description = "CIDR block for the VPC."
  type        = string
  default     = "10.20.0.0/16"
}

variable "ingress_cidrs" {
  description = "CIDR ranges allowed to reach the ALB on port 80."
  type        = list(string)
  default     = ["0.0.0.0/0"]
}

# --- Application ------------------------------------------------------------

variable "container_port" {
  description = "Port the sigil-server container listens on."
  type        = number
  default     = 8080
}

variable "image_tag" {
  description = "Image tag to deploy from the server ECR repository."
  type        = string
  default     = "latest"
}

variable "issuer_host" {
  description = "Public host used to build the issuer did:web identifier."
  type        = string
  default     = "issuer.example"
}

variable "rust_log" {
  description = "RUST_LOG filter for the server."
  type        = string
  default     = "info"
}

variable "task_cpu" {
  description = "Fargate task CPU units (1024 = 1 vCPU)."
  type        = number
  default     = 256
}

variable "task_memory" {
  description = "Fargate task memory in MiB."
  type        = number
  default     = 512
}

variable "service_desired_count" {
  description = "Number of server tasks to run."
  type        = number
  default     = 2
}

variable "log_retention_days" {
  description = "CloudWatch log retention for the server log group."
  type        = number
  default     = 30
}

# --- Database ---------------------------------------------------------------

variable "db_engine_version" {
  description = "Postgres engine version for RDS."
  type        = string
  default     = "16.4"
}

variable "db_instance_class" {
  description = "RDS instance class."
  type        = string
  default     = "db.t4g.micro"
}

variable "db_allocated_storage" {
  description = "Initial RDS storage in GiB."
  type        = number
  default     = 20
}

variable "db_max_allocated_storage" {
  description = "Upper bound for RDS storage autoscaling in GiB."
  type        = number
  default     = 100
}

variable "db_name" {
  description = "Initial database name."
  type        = string
  default     = "sigil"
}

variable "db_username" {
  description = "Master username for RDS."
  type        = string
  default     = "sigil"
}

variable "db_password" {
  description = "Master password for RDS. Supply via a secret manager or TF_VAR; never commit a real value."
  type        = string
  sensitive   = true
}

variable "db_multi_az" {
  description = "Whether to run RDS in multi-AZ mode."
  type        = bool
  default     = false
}

variable "db_backup_retention_days" {
  description = "Automated backup retention in days."
  type        = number
  default     = 7
}
