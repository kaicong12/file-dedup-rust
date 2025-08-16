"use client";

import React, { useState, useEffect, useRef, useCallback } from "react";
import LoginForm from "../components/LoginForm";
import Dashboard from "../components/Dashboard";

interface User {
  id: string;
  name: string;
  email: string;
}

interface Notification {
  id: number;
  type: "success" | "error" | "info";
  message: string;
}

interface FileInfo {
  name: string;
  size: number;
  hash?: string;
}

interface DuplicateGroup {
  files: FileInfo[];
  wasted_space: number;
}

interface JobResults {
  duplicate_groups: DuplicateGroup[];
  total_files: number;
  wasted_space: number;
}

interface Job {
  id: string;
  status: "pending" | "processing" | "completed" | "failed";
  created_at: string;
  total_files?: number;
  duplicate_groups?: number;
  wasted_space?: number;
  progress?: number;
  results?: JobResults;
}

interface WebSocketMessage {
  type: "job_status_update" | "job_completed" | "job_failed";
  job_id: string;
  status?: Partial<Job>;
  error?: string;
}

const API_BASE_URL = process.env.REACT_APP_API_URL || "http://localhost:8080";
const WS_URL = process.env.REACT_APP_WS_URL || "ws://localhost:8080/ws";

const FileDeduplicationSystem = () => {
  const [jobs, setJobs] = useState<Job[]>([]);
  const [currentJob, setCurrentJob] = useState<Job | null>(null);
  const [uploadProgress, setUploadProgress] = useState<number>(0);
  const [isUploading, setIsUploading] = useState<boolean>(false);
  const [authToken, setAuthToken] = useState<string | null>(null);
  const [user, setUser] = useState<User | null>(null);
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const wsRef = useRef<WebSocket | null>(null);

  // Initialize auth token from localStorage on client side only
  useEffect(() => {
    const token = localStorage.getItem("auth_token");
    setAuthToken(token);
  }, []);

  const fetchJobs = useCallback(async (): Promise<void> => {
    if (!authToken) return;

    try {
      const response = await fetch(`${API_BASE_URL}/jobs`, {
        headers: { Authorization: `Bearer ${authToken}` },
      });
      if (response.ok) {
        const jobsData: Job[] = await response.json();
        setJobs(jobsData);
      }
    } catch (error) {
      console.error("Failed to fetch jobs:", error);
    }
  }, [authToken]);

  const handleWebSocketMessage = useCallback(
    (data: WebSocketMessage): void => {
      switch (data.type) {
        case "job_status_update":
          setJobs((prev) =>
            prev.map((job) =>
              job.id === data.job_id ? { ...job, ...data.status } : job
            )
          );
          break;
        case "job_completed":
          addNotification(
            "success",
            `Job ${data.job_id} completed successfully`
          );
          fetchJobs();
          break;
        case "job_failed":
          addNotification("error", `Job ${data.job_id} failed: ${data.error}`);
          break;
        default:
          break;
      }
    },
    [fetchJobs]
  );

  // WebSocket connection for real-time updates
  const connectWebSocket = useCallback((): void => {
    if (wsRef.current || !authToken) return;

    wsRef.current = new WebSocket(`${WS_URL}?token=${authToken}`);

    wsRef.current.onmessage = (event: MessageEvent) => {
      const data: WebSocketMessage = JSON.parse(event.data);
      handleWebSocketMessage(data);
    };

    wsRef.current.onclose = () => {
      wsRef.current = null;
      // Reconnect after 3 seconds if still authenticated
      if (authToken) {
        setTimeout(connectWebSocket, 3000);
      }
    };
  }, [authToken, handleWebSocketMessage]);

  const logout = useCallback((): void => {
    setAuthToken(null);
    setUser(null);
    localStorage.removeItem("auth_token");
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }
  }, []);

  const fetchUserProfile = useCallback(async (): Promise<void> => {
    if (!authToken) return;

    try {
      const response = await fetch(`${API_BASE_URL}/user/profile`, {
        headers: { Authorization: `Bearer ${authToken}` },
      });
      if (response.ok) {
        const userData: User = await response.json();
        setUser(userData);
        fetchJobs();
      } else {
        logout();
      }
    } catch (error) {
      console.error("Failed to fetch user profile:", error);
      logout();
    }
  }, [authToken, fetchJobs, logout]);

  // Authentication check
  useEffect(() => {
    if (authToken) {
      fetchUserProfile();
      connectWebSocket();
    }
  }, [authToken, connectWebSocket, fetchUserProfile]);

  const login = async (email: string, password: string): Promise<void> => {
    try {
      const response = await fetch(`${API_BASE_URL}/auth/login`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email, password }),
      });

      if (response.ok) {
        const { token, user: userData }: { token: string; user: User } =
          await response.json();
        console.log({ token });
        setAuthToken(token);
        setUser(userData);
        localStorage.setItem("auth_token", token);
        connectWebSocket();
        fetchJobs();
      } else {
        const error = await response.json();
        addNotification("error", error.message || "Login failed");
      }
    } catch (error) {
      console.error("Login failed:", error);
      addNotification("error", "Login failed");
    }
  };

  const addNotification = (
    type: Notification["type"],
    message: string
  ): void => {
    const id = Date.now();
    setNotifications((prev) => [...prev, { id, type, message }]);
    setTimeout(() => {
      setNotifications((prev) => prev.filter((n) => n.id !== id));
    }, 5000);
  };

  const uploadFiles = async (files: File[]): Promise<void> => {
    if (!files.length || !authToken) return;

    // Validate file types and sizes
    const maxFileSize = 100 * 1024 * 1024; // 100MB
    const allowedTypes = [
      "image/",
      "text/",
      "application/pdf",
      "application/msword",
    ];

    for (const file of files) {
      if (file.size > maxFileSize) {
        addNotification("error", `File ${file.name} is too large (max 100MB)`);
        return;
      }

      if (!allowedTypes.some((type) => file.type.startsWith(type))) {
        addNotification("error", `File type ${file.type} is not allowed`);
        return;
      }
    }

    setIsUploading(true);
    setUploadProgress(0);

    const formData = new FormData();
    for (const file of files) {
      formData.append("files", file);
    }

    try {
      const response = await fetch(`${API_BASE_URL}/jobs/create`, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${authToken}`,
        },
        body: formData,
      });

      if (response.ok) {
        const job: Job = await response.json();
        setCurrentJob(job);
        addNotification("success", `Deduplication job created: ${job.id}`);
        fetchJobs();
      } else {
        const error = await response.json();
        addNotification("error", error.message || "Upload failed");
      }
    } catch (error) {
      console.error("Upload failed:", error);
      addNotification("error", "Upload failed");
    } finally {
      setIsUploading(false);
      setUploadProgress(0);
    }
  };

  const deleteJob = async (jobId: string): Promise<void> => {
    if (!authToken) return;

    try {
      const response = await fetch(`${API_BASE_URL}/jobs/${jobId}`, {
        method: "DELETE",
        headers: { Authorization: `Bearer ${authToken}` },
      });

      if (response.ok) {
        setJobs((prev) => prev.filter((job) => job.id !== jobId));
        addNotification("success", "Job deleted successfully");
      }
    } catch (error) {
      console.error("Failed to delete job:", error);
      addNotification("error", "Failed to delete job");
    }
  };

  const downloadResults = async (jobId: string): Promise<void> => {
    if (!authToken) return;

    try {
      const response = await fetch(`${API_BASE_URL}/jobs/${jobId}/results`, {
        headers: { Authorization: `Bearer ${authToken}` },
      });

      if (response.ok) {
        const blob = await response.blob();
        const url = window.URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = `deduplication-results-${jobId}.json`;
        a.click();
        window.URL.revokeObjectURL(url);
      }
    } catch (error) {
      console.error("Failed to download results:", error);
      addNotification("error", "Failed to download results");
    }
  };

  // Show login form if not authenticated
  if (!authToken) {
    return <LoginForm onLogin={login} />;
  }

  // Show dashboard if authenticated
  return (
    <Dashboard
      user={user}
      jobs={jobs}
      currentJob={currentJob}
      uploadProgress={uploadProgress}
      isUploading={isUploading}
      notifications={notifications}
      onLogout={logout}
      onUploadFiles={uploadFiles}
      onDeleteJob={deleteJob}
      onDownloadResults={downloadResults}
      onFetchJobs={fetchJobs}
      onSetCurrentJob={setCurrentJob}
    />
  );
};

export default FileDeduplicationSystem;
