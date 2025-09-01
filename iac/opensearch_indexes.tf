# OpenSearch indexes for file deduplication service

# File embeddings index for documents (text files, PDFs, etc.)
resource "opensearch_index" "file_embeddings" {
  name = "file-embeddings"
  body = jsonencode({
    settings = {
      index = {
        number_of_shards   = 1
        number_of_replicas = 1
        knn                = true
        "knn.algo_param.ef_search" = 100
      }
    }
    mappings = {
      properties = {
        file_id = {
          type = "integer"
        }
        file_name = {
          type = "text"
          fields = {
            keyword = {
              type = "keyword"
            }
          }
        }
        sha256_hash = {
          type = "keyword"
        }
        embedding = {
          type       = "dense_vector"
          dims       = 1536
          index      = true
          similarity = "cosine"
        }
        created_at = {
          type = "date"
        }
      }
    }
  })
}

# Image embeddings index for image files
resource "opensearch_index" "image_embeddings" {
  name = "image-embeddings"
  body = jsonencode({
    settings = {
      index = {
        number_of_shards   = 1
        number_of_replicas = 1
        knn                = true
        "knn.algo_param.ef_search" = 100
      }
    }
    mappings = {
      properties = {
        file_id = {
          type = "integer"
        }
        file_name = {
          type = "text"
          fields = {
            keyword = {
              type = "keyword"
            }
          }
        }
        sha256_hash = {
          type = "keyword"
        }
        embedding = {
          type       = "dense_vector"
          dims       = 1024
          index      = true
          similarity = "cosine"
        }
        created_at = {
          type = "date"
        }
      }
    }
  })
}

# Index template for file embeddings (optional - for automatic index creation)
resource "opensearch_index_template" "file_embeddings_template" {
  name = "file-embeddings-template"
  body = jsonencode({
    index_patterns = ["file-embeddings*"]
    template = {
      settings = {
        index = {
          number_of_shards   = 1
          number_of_replicas = 1
          knn                = true
          "knn.algo_param.ef_search" = 100
        }
      }
      mappings = {
        properties = {
          file_id = {
            type = "integer"
          }
          file_name = {
            type = "text"
            fields = {
              keyword = {
                type = "keyword"
              }
            }
          }
          sha256_hash = {
            type = "keyword"
          }
          embedding = {
            type       = "dense_vector"
            dims       = 1536
            index      = true
            similarity = "cosine"
          }
          created_at = {
            type = "date"
          }
        }
      }
    }
  })
}

# Index template for image embeddings (optional - for automatic index creation)
resource "opensearch_index_template" "image_embeddings_template" {
  name = "image-embeddings-template"
  body = jsonencode({
    index_patterns = ["image-embeddings*"]
    template = {
      settings = {
        index = {
          number_of_shards   = 1
          number_of_replicas = 1
          knn                = true
          "knn.algo_param.ef_search" = 100
        }
      }
      mappings = {
        properties = {
          file_id = {
            type = "integer"
          }
          file_name = {
            type = "text"
            fields = {
              keyword = {
                type = "keyword"
              }
            }
          }
          sha256_hash = {
            type = "keyword"
          }
          embedding = {
            type       = "dense_vector"
            dims       = 1024
            index      = true
            similarity = "cosine"
          }
          created_at = {
            type = "date"
          }
        }
      }
    }
  })
}

# Output the index names for reference
output "file_embeddings_index_name" {
  description = "Name of the file embeddings index"
  value       = opensearch_index.file_embeddings.name
}

output "image_embeddings_index_name" {
  description = "Name of the image embeddings index"
  value       = opensearch_index.image_embeddings.name
}
