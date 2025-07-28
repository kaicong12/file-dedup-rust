"use client";

import React, { useState, useEffect, useRef, useCallback } from "react";
import {
  Upload,
  FileText,
  Trash2,
  HardDrive,
  Clock,
  CheckCircle,
  XCircle,
  RefreshCw,
  Eye,
  Download,
  Shield,
} from "lucide-react";

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

const API_BASE_URL =
  process.env.REACT_APP_API_URL || "http://localhost:8080/api";
const WS_URL = process.env.REACT_APP_WS_URL || "ws://localhost:8080/ws";

const FileDeduplicationSystem = () => {
  const [jobs, setJobs] = useState<Job[]>([]);
  const [currentJob, setCurrentJob] = useState<Job | null>(null);
  const [uploadProgress, setUploadProgress] = useState<number>(0);
  const [isUploading, setIsUploading] = useState<boolean>(false);
  const [authToken, setAuthToken] = useState<string | null>(
    localStorage.getItem("auth_token")
  );
  const [user, setUser] = useState<User | null>(null);
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const wsRef = useRef<WebSocket | null>(null);

  const fetchJobs = useCallback(async (): Promise<void> => {
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
    if (wsRef.current) return;

    wsRef.current = new WebSocket(`${WS_URL}?token=${authToken}`);

    wsRef.current.onmessage = (event: MessageEvent) => {
      const data: WebSocketMessage = JSON.parse(event.data);
      handleWebSocketMessage(data);
    };

    wsRef.current.onclose = () => {
      // Reconnect after 3 seconds
      setTimeout(connectWebSocket, 3000);
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
        setAuthToken(token);
        setUser(userData);
        localStorage.setItem("auth_token", token);
        connectWebSocket();
        fetchJobs();
      } else {
        const error = await response.json();
        addNotification("error", error.message);
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
    if (!files.length) return;

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
        addNotification("error", error.message);
      }
    } catch (error) {
      console.error("Upload failed:", error);
      addNotification("error", "Upload failed");
    } finally {
      setIsUploading(false);
      setUploadProgress(0);
    }
  };

  const handleFileUpload = (
    event: React.ChangeEvent<HTMLInputElement>
  ): void => {
    if (!event.target.files) return;
    const files = Array.from(event.target.files);
    uploadFiles(files);
  };

  const deleteJob = async (jobId: string): Promise<void> => {
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

  const getStatusIcon = (status: Job["status"]): React.ReactElement => {
    switch (status) {
      case "pending":
        return <Clock className="text-yellow-500" size={16} />;
      case "processing":
        return <RefreshCw className="text-blue-500 animate-spin" size={16} />;
      case "completed":
        return <CheckCircle className="text-green-500" size={16} />;
      case "failed":
        return <XCircle className="text-red-500" size={16} />;
      default:
        return <Clock className="text-gray-500" size={16} />;
    }
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return "0 Bytes";
    const k = 1024;
    const sizes = ["Bytes", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
  };

  // Login Form Component
  const LoginForm = (): React.ReactElement => {
    const [email, setEmail] = useState<string>("");
    const [password, setPassword] = useState<string>("");

    const handleSubmit = (): void => {
      if (email && password) {
        login(email, password);
      }
    };

    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="max-w-md w-full bg-white rounded-lg shadow-lg p-8">
          <div className="text-center mb-8">
            <Shield className="mx-auto h-12 w-12 text-blue-600 mb-4" />
            <h2 className="text-2xl font-bold text-gray-800">
              Secure File Deduplication
            </h2>
            <p className="text-gray-600 mt-2">Sign in to your account</p>
          </div>

          <div className="space-y-6">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Email Address
              </label>
              <input
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                required
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Password
              </label>
              <input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                required
              />
            </div>

            <button
              type="button"
              onClick={handleSubmit}
              className="w-full bg-blue-600 text-white py-2 px-4 rounded-lg hover:bg-blue-700 transition-colors"
            >
              Sign In
            </button>
          </div>
        </div>
      </div>
    );
  };

  // Notifications Component
  const Notifications = (): React.ReactElement => {
    const getNotificationStyles = (type: Notification["type"]): string => {
      switch (type) {
        case "success":
          return "bg-green-100 text-green-800";
        case "error":
          return "bg-red-100 text-red-800";
        default:
          return "bg-blue-100 text-blue-800";
      }
    };

    return (
      <div className="fixed top-4 right-4 z-50 space-y-2">
        {notifications.map((notification) => (
          <div
            key={notification.id}
            className={`p-4 rounded-lg shadow-lg max-w-sm ${getNotificationStyles(
              notification.type
            )}`}
          >
            {notification.message}
          </div>
        ))}
      </div>
    );
  };

  if (!authToken) {
    return <LoginForm />;
  }

  return (
    <div className="max-w-7xl mx-auto p-6 bg-gray-50 min-h-screen">
      <Notifications />

      {/* Header */}
      <div className="bg-white rounded-lg shadow-lg p-6 mb-6">
        <div className="flex justify-between items-center">
          <h1 className="text-3xl font-bold text-gray-800 flex items-center gap-3">
            <HardDrive className="text-blue-600" />
            Secure File Deduplication System
          </h1>
          <div className="flex items-center gap-4">
            <span className="text-gray-600">Welcome, {user?.name}</span>
            <button
              onClick={logout}
              className="bg-red-600 text-white px-4 py-2 rounded-lg hover:bg-red-700"
            >
              Logout
            </button>
          </div>
        </div>
      </div>

      {/* Upload Section */}
      <div className="bg-white rounded-lg shadow-lg p-6 mb-6">
        <h2 className="text-xl font-semibold text-gray-800 mb-4">
          Upload Files for Deduplication
        </h2>

        <div
          className="border-2 border-dashed border-gray-300 rounded-lg p-8 text-center hover:border-blue-400 transition-colors cursor-pointer"
          role="button"
          tabIndex={0}
          onClick={() => fileInputRef.current?.click()}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              fileInputRef.current?.click();
            }
          }}
        >
          <Upload className="mx-auto h-12 w-12 text-gray-400 mb-4" />
          <p className="text-lg text-gray-600">
            Click to upload files or drag and drop
          </p>
          <p className="text-sm text-gray-500 mt-2">
            Supported: Images, Documents, PDFs (Max 100MB per file)
          </p>
          {isUploading && (
            <div className="mt-4">
              <div className="w-full bg-gray-200 rounded-full h-2">
                <div
                  className="bg-blue-600 h-2 rounded-full transition-all duration-300"
                  style={{ width: `${uploadProgress}%` }}
                />
              </div>
              <p className="text-sm text-gray-500 mt-2">
                Uploading... {uploadProgress}%
              </p>
            </div>
          )}
        </div>
        <input
          ref={fileInputRef}
          type="file"
          multiple
          onChange={handleFileUpload}
          className="hidden"
          disabled={isUploading}
        />
      </div>

      {/* Jobs List */}
      <div className="bg-white rounded-lg shadow-lg p-6">
        <div className="flex justify-between items-center mb-6">
          <h2 className="text-xl font-semibold text-gray-800">
            Deduplication Jobs
          </h2>
          <button
            onClick={fetchJobs}
            className="bg-blue-600 text-white px-4 py-2 rounded-lg hover:bg-blue-700 flex items-center gap-2"
          >
            <RefreshCw size={16} />
            Refresh
          </button>
        </div>

        {jobs.length === 0 ? (
          <div className="text-center py-12">
            <FileText className="mx-auto h-16 w-16 text-gray-300 mb-4" />
            <p className="text-gray-500">
              No deduplication jobs yet. Upload some files to get started.
            </p>
          </div>
        ) : (
          <div className="space-y-4">
            {jobs.map((job) => (
              <div
                key={job.id}
                className="border border-gray-200 rounded-lg p-4"
              >
                <div className="flex justify-between items-start mb-3">
                  <div className="flex items-center gap-3">
                    {getStatusIcon(job.status)}
                    <div>
                      <h3 className="font-medium text-gray-800">
                        Job #{job.id}
                      </h3>
                      <p className="text-sm text-gray-500">
                        Created: {new Date(job.created_at).toLocaleString()}
                      </p>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    {job.status === "completed" && (
                      <>
                        <button
                          onClick={() => downloadResults(job.id)}
                          className="text-sm bg-green-100 text-green-700 px-3 py-1 rounded hover:bg-green-200 flex items-center gap-1"
                        >
                          <Download size={14} />
                          Download
                        </button>
                        <button
                          onClick={() =>
                            setCurrentJob(
                              currentJob?.id === job.id ? null : job
                            )
                          }
                          className="text-sm bg-blue-100 text-blue-700 px-3 py-1 rounded hover:bg-blue-200 flex items-center gap-1"
                        >
                          <Eye size={14} />
                          View
                        </button>
                      </>
                    )}
                    <button
                      onClick={() => deleteJob(job.id)}
                      className="text-sm bg-red-100 text-red-700 px-3 py-1 rounded hover:bg-red-200 flex items-center gap-1"
                    >
                      <Trash2 size={14} />
                      Delete
                    </button>
                  </div>
                </div>

                <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                  <div>
                    <span className="text-gray-500">Status:</span>
                    <span className="ml-2 font-medium capitalize">
                      {job.status}
                    </span>
                  </div>
                  <div>
                    <span className="text-gray-500">Files:</span>
                    <span className="ml-2 font-medium">
                      {job.total_files || 0}
                    </span>
                  </div>
                  <div>
                    <span className="text-gray-500">Duplicates:</span>
                    <span className="ml-2 font-medium">
                      {job.duplicate_groups || 0}
                    </span>
                  </div>
                  <div>
                    <span className="text-gray-500">Saved Space:</span>
                    <span className="ml-2 font-medium">
                      {job.wasted_space
                        ? formatFileSize(job.wasted_space)
                        : "0 B"}
                    </span>
                  </div>
                </div>

                {job.status === "processing" && job.progress && (
                  <div className="mt-3">
                    <div className="flex justify-between text-sm mb-1">
                      <span>Progress</span>
                      <span>{job.progress}%</span>
                    </div>
                    <div className="w-full bg-gray-200 rounded-full h-2">
                      <div
                        className="bg-blue-600 h-2 rounded-full transition-all duration-300"
                        style={{ width: `${job.progress}%` }}
                      />
                    </div>
                  </div>
                )}

                {currentJob?.id === job.id &&
                  job.status === "completed" &&
                  job.results && (
                    <div className="mt-4 p-4 bg-gray-50 rounded-lg">
                      <h4 className="font-medium text-gray-800 mb-3">
                        Deduplication Results
                      </h4>
                      {job.results.duplicate_groups?.map((group, index) => (
                        <div
                          key={`group-${job.id}-${index}`}
                          className="mb-3 p-3 bg-white rounded border"
                        >
                          <div className="flex justify-between items-center mb-2">
                            <span className="font-medium">
                              Group {index + 1}
                            </span>
                            <span className="text-sm text-gray-500">
                              {group.files?.length} files,{" "}
                              {formatFileSize(group.wasted_space)} wasted
                            </span>
                          </div>
                          <div className="space-y-1">
                            {group.files?.slice(0, 3).map((file, fileIndex) => (
                              <div
                                key={`file-${job.id}-${index}-${fileIndex}-${file.name}`}
                                className="text-sm text-gray-600 flex items-center gap-2"
                              >
                                <FileText size={14} />
                                {file.name}
                              </div>
                            ))}
                            {group.files?.length > 3 && (
                              <div className="text-sm text-gray-500">
                                ... and {group.files.length - 3} more files
                              </div>
                            )}
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default FileDeduplicationSystem;
