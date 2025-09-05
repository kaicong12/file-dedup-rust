export interface FileInfo {
  name: string;
  size: number;
  hash?: string;
}

export interface DuplicateGroup {
  files: FileInfo[];
  wasted_space: number;
}

export interface JobResults {
  duplicate_groups: DuplicateGroup[];
  total_files: number;
  wasted_space: number;
}

export interface Job {
  job_id: string;
  status: "pending" | "processing" | "completed" | "failed";
  created_at: string;
  total_files?: number;
  duplicate_groups?: number;
  wasted_space?: number;
  progress?: number;
  results?: JobResults;
}
