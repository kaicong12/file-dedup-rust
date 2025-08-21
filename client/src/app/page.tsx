"use client";

import React, { useState, useRef, useCallback, useEffect } from "react";
import LoginForm from "../components/LoginForm";
import Dashboard from "../components/Dashboard";
import { useAuth } from "../contexts/AuthContext";

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
  const { isAuthenticated, username, login, logout } = useAuth();
  const [jobs, setJobs] = useState<Job[]>([]);
  const [currentJob, setCurrentJob] = useState<Job | null>(null);
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const wsRef = useRef<WebSocket | null>(null);

  const fetchJobs = useCallback(async (): Promise<void> => {
    if (!isAuthenticated) return;

    const token = localStorage.getItem("auth_token");
    if (!token) return;

    try {
      const response = await fetch(`${API_BASE_URL}/jobs`, {
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
      });
      if (response.ok) {
        const jobsData: Job[] = await response.json();
        setJobs(jobsData);
      }
    } catch (error) {
      console.error("Failed to fetch jobs:", error);
    }
  }, [isAuthenticated]);

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
    if (wsRef.current || !isAuthenticated) return;

    // For cookie-based auth, we don't need to pass token in URL
    wsRef.current = new WebSocket(WS_URL);

    wsRef.current.onmessage = (event: MessageEvent) => {
      const data: WebSocketMessage = JSON.parse(event.data);
      handleWebSocketMessage(data);
    };

    wsRef.current.onclose = () => {
      wsRef.current = null;
      // Reconnect after 3 seconds if still authenticated
      if (isAuthenticated) {
        setTimeout(connectWebSocket, 3000);
      }
    };
  }, [isAuthenticated, handleWebSocketMessage]);

  const handleLogin = async (
    email: string,
    password: string
  ): Promise<void> => {
    try {
      await login(email, password);
      addNotification("success", "Logged in successfully");
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : "Login failed";
      addNotification("error", errorMessage);
      throw error;
    }
  };

  const handleLogout = useCallback(async (): Promise<void> => {
    try {
      await logout();

      // Clear local state
      setJobs([]);
      setCurrentJob(null);

      // Close WebSocket connection
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }

      addNotification("success", "Logged out successfully");
    } catch (error) {
      console.error("Logout failed:", error);
      addNotification("error", "Logout failed");
    }
  }, [logout]);

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

  const deleteJob = async (jobId: string): Promise<void> => {
    if (!isAuthenticated) return;

    const token = localStorage.getItem("auth_token");
    if (!token) return;

    try {
      const response = await fetch(`${API_BASE_URL}/jobs/${jobId}`, {
        method: "DELETE",
        headers: {
          Authorization: `Bearer ${token}`,
        },
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
    if (!isAuthenticated) return;

    const token = localStorage.getItem("auth_token");
    if (!token) return;

    try {
      const response = await fetch(`${API_BASE_URL}/jobs/${jobId}/results`, {
        headers: {
          Authorization: `Bearer ${token}`,
        },
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

  // Connect WebSocket and fetch jobs when authenticated
  useEffect(() => {
    if (isAuthenticated) {
      connectWebSocket();
      fetchJobs();
    }
  }, [isAuthenticated, connectWebSocket, fetchJobs]);

  // Show login form if not authenticated
  if (!isAuthenticated) {
    return <LoginForm onLogin={handleLogin} />;
  }

  // Show dashboard if authenticated
  return (
    <Dashboard
      username={username}
      jobs={jobs}
      currentJob={currentJob}
      notifications={notifications}
      onLogout={handleLogout}
      onDeleteJob={deleteJob}
      onDownloadResults={downloadResults}
      onFetchJobs={fetchJobs}
      onSetCurrentJob={setCurrentJob}
      onAddNotification={addNotification}
    />
  );
};

export default FileDeduplicationSystem;
