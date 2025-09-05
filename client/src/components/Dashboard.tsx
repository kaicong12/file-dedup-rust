"use client";

import React, { useState, useCallback } from "react";
import {
  FileText,
  Trash2,
  HardDrive,
  Clock,
  CheckCircle,
  XCircle,
  RefreshCw,
  Eye,
  Download,
} from "lucide-react";
import MultipartUpload from "./MultipartUpload";
import { Job } from "@/app/type";

interface Notification {
  id: number;
  type: "success" | "error" | "info";
  message: string;
}

interface UploadResult {
  fileName: string;
  success: boolean;
  error?: string;
}

interface DashboardProps {
  username: string;
  jobs: Job[];
  currentJob: Job | null;
  notifications: Notification[];
  onLogout: () => void;
  onDeleteJob: (jobId: string) => Promise<void>;
  onDownloadResults: (jobId: string) => Promise<void>;
  onFetchJobs: () => Promise<void>;
  onSetCurrentJob: (job: Job | null) => void;
  onAddNotification: (type: Notification["type"], message: string) => void;
}

const Dashboard: React.FC<DashboardProps> = ({
  username,
  jobs,
  currentJob,
  notifications,
  onLogout,
  onDeleteJob,
  onDownloadResults,
  onFetchJobs,
  onSetCurrentJob,
  onAddNotification,
}) => {
  // Internal upload state
  const [uploadProgress, setUploadProgress] = useState<number>(0);
  const [isUploading, setIsUploading] = useState<boolean>(false);

  // Upload handlers
  const handleUploadComplete = useCallback(
    (results: UploadResult[]) => {
      const successCount = results.filter((r) => r.success).length;
      const failCount = results.filter((r) => !r.success).length;

      if (successCount > 0) {
        onAddNotification(
          "success",
          `Successfully uploaded ${successCount} file(s)`
        );
        // Refresh jobs after successful upload
        onFetchJobs();
      }

      if (failCount > 0) {
        onAddNotification("error", `Failed to upload ${failCount} file(s)`);
      }

      setIsUploading(false);
    },
    [onAddNotification, onFetchJobs]
  );

  const handleUploadProgress = useCallback((progress: number) => {
    setUploadProgress(progress);
  }, []);

  const handleUploadStart = useCallback(() => {
    setIsUploading(true);
    setUploadProgress(0);
  }, []);

  const handleUploadError = useCallback(
    (error: string) => {
      onAddNotification("error", error);
      setIsUploading(false);
      setUploadProgress(0);
    },
    [onAddNotification]
  );

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
            <span className="text-gray-600">Welcome, {username}</span>
            <button
              onClick={onLogout}
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

        <MultipartUpload
          onUploadComplete={handleUploadComplete}
          onUploadProgress={handleUploadProgress}
          onUploadStart={handleUploadStart}
          onUploadError={handleUploadError}
          isAuthenticated={!!username}
        />

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

      {/* Jobs List */}
      <div className="bg-white rounded-lg shadow-lg p-6">
        <div className="flex justify-between items-center mb-6">
          <h2 className="text-xl font-semibold text-gray-800">
            Deduplication Jobs
          </h2>
          <button
            onClick={onFetchJobs}
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
                key={job.job_id}
                className="border border-gray-200 rounded-lg p-4"
              >
                <div className="flex justify-between items-start mb-3">
                  <div className="flex items-center gap-3">
                    {getStatusIcon(job.status)}
                    <div>
                      <h3 className="font-medium text-gray-800">
                        Job #{job.job_id}
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
                          onClick={() => onDownloadResults(job.job_id)}
                          className="text-sm bg-green-100 text-green-700 px-3 py-1 rounded hover:bg-green-200 flex items-center gap-1"
                        >
                          <Download size={14} />
                          Download
                        </button>
                        <button
                          onClick={() =>
                            onSetCurrentJob(
                              currentJob?.job_id === job.job_id ? null : job
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
                      onClick={() => onDeleteJob(job.job_id)}
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

                {currentJob?.job_id === job.job_id &&
                  job.status === "completed" &&
                  job.results && (
                    <div className="mt-4 p-4 bg-gray-50 rounded-lg">
                      <h4 className="font-medium text-gray-800 mb-3">
                        Deduplication Results
                      </h4>
                      {job.results.duplicate_groups?.map((group, index) => (
                        <div
                          key={`group-${job.job_id}-${index}`}
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
                                key={`file-${job.job_id}-${index}-${fileIndex}-${file.name}`}
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

export default Dashboard;
