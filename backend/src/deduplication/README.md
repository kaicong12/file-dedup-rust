## Goals

- Efficiently block exact duplicates during file uploads.
- Suggest near-duplicates for moderators or automated merging rules.
- Minimize expensive bucket scans when retrieving near-duplicate files.

## Workflow

1. **Exact Duplicate Check**: On file upload, check for exact duplicates using file hashes.
2. **Embedding Creation**: Determine the file format, preprocess the file (e.g., create image embeddings or vector embeddings for documents).
3. **Near-Duplicate Search**: Use cosine similarity to identify near-duplicate files via OpenSearch.
4. **Vector Storage**: Upload the current file's vector embedding to OpenSearch.

## Database Schema

The system requires two tables:

1. **Cluster Table**: Stores clusters of similar files. Each cluster groups files with high similarity.
2. **File Table**: Stores individual file details. This is the table users interact with, while the cluster table is used for retrieving near-duplicate files.

### Cluster Table Schema

- `cluster_id`: Unique identifier for the cluster.
- `intra_similarity_score`: Average similarity score of files within the cluster.
- `created_at`: Timestamp when the cluster was created.

### File Table Schema (mirrored in OpenSearch)

- `file_id`: Unique identifier for the file.
- `file_name`: Name of the file.
- `cluster_id`: ID of the cluster the file belongs to.
- `created_at`: Timestamp when the file was uploaded.
- `embeddings`: Vector representation of the file.
- `sha256_hash`: SHA-256 hash for exact match

## Backend Worker Logic

The backend worker is responsible for processing files upon upload. Its workflow is as follows:

1. **Compute Embedding**: Generate the file's embedding and use OpenSearch to find the most similar file.
2. **Cluster Assignment**:
   - If the most similar file has a similarity score below `0.75`, create a new cluster and assign the file to it.
   - Otherwise, retrieve all files in the top cluster and compute the updated `intra_similarity_score` after adding the new file.
3. **Cluster Validation**:
   - If the updated `intra_similarity_score` falls below a predefined threshold, create a new cluster for the file.
   - Otherwise, add the file to the existing cluster.

This approach ensures efficient handling of both exact and near-duplicate files while maintaining cluster integrity.
