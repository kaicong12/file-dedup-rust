# Creates an encryption security policy
resource "aws_opensearchserverless_security_policy" "encryption_policy" {
    name        = "opensearch-encryption-policy"
    type        = "encryption"
    description = "encryption policy for ${var.collection_name}"
    policy = jsonencode({
        Rules = [
            {
                Resource = [
                    "collection/${var.collection_name}"
                ],
                ResourceType = "collection"
            }
        ],
        AWSOwnedKey = true
    })
}

# Creates a collection
resource "aws_opensearchserverless_collection" "collection" {
    name = var.collection_name

    depends_on = [aws_opensearchserverless_security_policy.encryption_policy]
}

# Creates a network security policy
resource "aws_opensearchserverless_security_policy" "network_policy" {
    name        = "opensearch-network-policy"
    type        = "network"
    description = "public access for dashboard, VPC access for collection endpoint"
    policy = jsonencode([
        {
            "AllowFromPublic": true,
            "Rules": [
            {
                "ResourceType": "collection",
                "Resource": [
                    "collection/${var.collection_name}"
                ]
            },
            {
                "ResourceType": "dashboard",
                "Resource": [
                    "collection/${var.collection_name}"
                ]
            }
            ]
        }
    ])
}

# Gets access to the effective Account ID in which Terraform is authorized
data "aws_caller_identity" "current" {}

# Creates a data access policy
resource "aws_opensearchserverless_access_policy" "data_access_policy" {
    name        = "opensearch-data-access-policy"
    type        = "data"
    description = "allow index and collection access"
    policy = jsonencode([
        {
            Rules = [
            {
                ResourceType = "index",
                Resource = [
                    "index/${var.collection_name}/*"
                ],
                Permission = [
                    "aoss:*"
                ]
            },
            {
                ResourceType = "collection",
                Resource = [
                    "collection/${var.collection_name}"
                ],
                Permission = [
                    "aoss:*"
                ]
            }
            ],
            Principal = [
                data.aws_caller_identity.current.arn
            ]
        }
    ])
}