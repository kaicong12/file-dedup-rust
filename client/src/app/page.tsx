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
  const [isAuthenticated, setIsAuthenticated] = useState<boolean>(false);
  const [user, setUser] = useState<User | null>(null);
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const wsRef = useRef<WebSocket | null>(null);

  const fetchJobs = useCallback(async (): Promise<void> => {
    if (!isAuthenticated) return;

    try {
      const response = await fetch(`${API_BASE_URL}/jobs`, {
        credentials: "include", // Include cookies
        headers: {
          "Content-Type": "application/json",
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

  const logout = useCallback(async (): Promise<void> => {
    try {
      // Call logout endpoint to clear the JWT cookie
      await fetch(`${API_BASE_URL}/auth/logout`, {
        method: "POST",
        credentials: "include", // Include cookies
        headers: {
          "Content-Type": "application/json",
        },
      });
    } catch (error) {
      console.error("Logout request failed:", error);
    } finally {
      // Clear local state regardless of API call success
      setIsAuthenticated(false);
      setUser(null);
      setJobs([]);
      setCurrentJob(null);

      // Close WebSocket connection
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }

      addNotification("success", "Logged out successfully");
    }
  }, []);

  const login = async (email: string, password: string): Promise<void> => {
    try {
      const response = await fetch(`${API_BASE_URL}/auth/login`, {
        method: "POST",
        credentials: "include", // Include cookies
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ email, password }),
      });

      if (response.ok) {
        const { user: userData }: { user: User } = await response.json();
        setUser(userData);
        setIsAuthenticated(true);
        connectWebSocket();
        fetchJobs();
        addNotification("success", "Logged in successfully");
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

  const checkAuthStatus = useCallback(async (): Promise<void> => {
    // For now, just set authenticated as true without calling /profile endpoint
    setUser({
      id: "1",
      name: "Test User",
      email: "test@example.com",
    });
    setIsAuthenticated(true);
    setIsLoading(false);
    // fetchJobs();
    // connectWebSocket();
  }, []);

  // Check authentication status on component mount
  useEffect(() => {
    checkAuthStatus();
  }, [checkAuthStatus]);

  const deleteJob = async (jobId: string): Promise<void> => {
    if (!isAuthenticated) return;

    try {
      const response = await fetch(`${API_BASE_URL}/jobs/${jobId}`, {
        method: "DELETE",
        credentials: "include", // Include cookies
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

    try {
      const response = await fetch(`${API_BASE_URL}/jobs/${jobId}/results`, {
        credentials: "include", // Include cookies
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

  // Show loading spinner while checking authentication
  if (isLoading) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto mb-4"></div>
          <p className="text-gray-600">Loading...</p>
        </div>
      </div>
    );
  }

  // Show login form if not authenticated
  if (!isAuthenticated) {
    return <LoginForm onLogin={login} />;
  }

  // Show dashboard if authenticated
  return (
    <Dashboard
      user={user!}
      jobs={jobs}
      currentJob={currentJob}
      notifications={notifications}
      onLogout={logout}
      onDeleteJob={deleteJob}
      onDownloadResults={downloadResults}
      onFetchJobs={fetchJobs}
      onSetCurrentJob={setCurrentJob}
      onAddNotification={addNotification}
    />
  );
};

export default FileDeduplicationSystem;
