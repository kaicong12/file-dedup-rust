CREATE TABLE Cluster (
    cluster_id SERIAL PRIMARY KEY, -- Unique identifier for the cluster
    intra_similarity_score FLOAT NOT NULL, -- Average similarity score of files within the cluster
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP -- Timestamp when the cluster was created
);

CREATE TABLE File (
    file_id SERIAL PRIMARY KEY, -- Unique identifier for the file
    file_name VARCHAR(255) NOT NULL, -- Name of the file
    cluster_id INT REFERENCES Cluster(cluster_id) ON DELETE SET NULL, -- ID of the cluster the file belongs to
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP, -- Timestamp when the file was uploaded
    sha256_hash CHAR(64) NOT NULL UNIQUE -- SHA-256 hash for exact duplicate matching
);

-- Create an index on the sha256_hash column in the File table
CREATE INDEX idx_sha256_hash ON File (sha256_hash);