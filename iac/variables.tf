variable "s3_bucket_name" {
  description = "The name of the S3 bucket"
  type = string
}

variable "aws_region" {
  description = "The AWS region to create things in."
  default     = "us-east-1"
}

variable "collection_name" {
  description = "Name of the OpenSearch Serverless collection."
  default     = "file-dedup-collection"
}

variable "created_vpc_id" {
  description = "The ID of the created VPC"
  type = string
}

variable "created_subnet_id" {
  description = "The ID of the created subnet"
  type = string
}