-- Create jobs table to store deduplication job data
CREATE TABLE IF NOT EXISTS jobs (
    job_id UUID PRIMARY KEY,
    file_id INTEGER NOT NULL REFERENCES File(file_id) ON DELETE CASCADE,
    file_name VARCHAR(255) NOT NULL,
    file_path VARCHAR(500),
    s3_key VARCHAR(500) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    error_message TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    completed_at TIMESTAMP WITH TIME ZONE
);

-- Create index on job status for efficient querying
CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status);

-- Create index on file_id for efficient lookups
CREATE INDEX IF NOT EXISTS idx_jobs_file_id ON jobs(file_id);

-- Create index on created_at for sorting
CREATE INDEX IF NOT EXISTS idx_jobs_created_at ON jobs(created_at DESC);
