# Closes #796: Terraform stub for the off-chain event indexer.
# Starter single-instance layout; KMS key policy and multi-AZ Postgres
# are follow-ups once the indexer's actual resource needs are known.

terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

variable "region" {
  type    = string
  default = "us-east-1"
}

variable "db_instance_class" {
  type    = string
  default = "db.t3.micro"
}

resource "aws_db_instance" "indexer_postgres" {
  identifier        = "propchain-indexer"
  engine            = "postgres"
  engine_version    = "15"
  instance_class    = var.db_instance_class
  allocated_storage = 20
  db_name           = "propchain_indexer"
  username          = "indexer"
  manage_master_user_password = true
  skip_final_snapshot = true
}

resource "aws_instance" "indexer_node" {
  ami           = "ami-0c55b159cbfafe1f0"
  instance_type = "t3.small"

  tags = {
    Name = "propchain-indexer"
  }
}

output "indexer_db_endpoint" {
  value = aws_db_instance.indexer_postgres.endpoint
}
