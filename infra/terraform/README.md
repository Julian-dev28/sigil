# Sigil — AWS reference infrastructure

Terraform describing a minimal, realistic AWS deployment of the Sigil API
server: an ECS Fargate service behind an Application Load Balancer, backed by
RDS Postgres, with ECR repositories for the container images.

This is a **reference skeleton**. It is valid, parameterized HCL that models the
intended topology so it can be read, adapted, and extended. Review it before
using it to run real workloads — in particular the items under
[Production hardening](#production-hardening).

## Topology

```
                Internet
                   │  :80
             ┌─────▼──────┐
             │    ALB     │  (public subnets, 2 AZs)
             └─────┬──────┘
                   │  :8080
          ┌────────▼─────────┐
          │  ECS Fargate     │  sigil-server tasks
          │  (private subnets)│
          └────────┬─────────┘
                   │  :5432
             ┌─────▼──────┐
             │ RDS Postgres│  (private subnets)
             └────────────┘

ECR: sigil/server, sigil/web   (images pushed by CI or `docker push`)
```

## Usage

```bash
cd infra/terraform

# Provide required/sensitive inputs (never commit real secrets):
export TF_VAR_db_password="$(openssl rand -base64 24)"

terraform init
terraform fmt -check
terraform validate
terraform plan  -var environment=dev
terraform apply -var environment=dev
```

Push images to the ECR repositories reported in the outputs, then update the
`image_tag` variable (or your deploy pipeline) to roll the service.

## Inputs

Key variables (see `variables.tf` for the full list and defaults):

| Variable                | Default          | Description                                   |
| ----------------------- | ---------------- | --------------------------------------------- |
| `project`               | `sigil`          | Resource name prefix.                         |
| `environment`           | `dev`            | Environment suffix; `prod` enables safeguards.|
| `aws_region`            | `us-east-1`      | Target region.                                |
| `vpc_cidr`              | `10.20.0.0/16`   | VPC address space.                            |
| `container_port`        | `8080`           | Server listen port.                           |
| `image_tag`             | `latest`         | Server image tag to deploy.                   |
| `service_desired_count` | `2`              | Number of server tasks.                       |
| `db_instance_class`     | `db.t4g.micro`   | RDS instance size.                            |
| `db_password`           | _(required)_     | RDS master password (sensitive).              |

## Outputs

`alb_dns_name`, `server_ecr_repository_url`, `web_ecr_repository_url`,
`ecs_cluster_name`, `ecs_service_name`, `database_endpoint`, `database_address`,
`vpc_id`.

## Production hardening

This skeleton favors readability over completeness. Before production use:

- **Task egress / image pulls** — private subnets have no NAT gateway or VPC
  endpoints here. Add a NAT gateway (or interface/gateway VPC endpoints for
  ECR, S3, CloudWatch Logs, and Secrets Manager) so Fargate tasks can pull
  images and ship logs.
- **TLS** — the ALB listener is HTTP/80. Add an ACM certificate and an HTTPS/443
  listener, redirecting 80 → 443.
- **Secrets** — inject `DATABASE_URL` / issuer key material via AWS Secrets
  Manager or SSM Parameter Store `secrets` in the task definition, not plain
  environment variables.
- **Remote state** — enable the S3 backend (with DynamoDB locking) in
  `main.tf`.
- **IAM** — the task role is intentionally empty; scope it to exactly what the
  service needs.
- **Autoscaling** — attach an Application Auto Scaling target/policy to the ECS
  service for load-based scaling.

## Files

| File           | Purpose                                            |
| -------------- | -------------------------------------------------- |
| `main.tf`      | Provider, networking, security groups, ECR, RDS, ECS/ALB. |
| `variables.tf` | Input variables with defaults and descriptions.    |
| `outputs.tf`   | Useful attributes after apply.                     |
