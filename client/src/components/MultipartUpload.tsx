"use client";

import React, { useState, useCallback } from "react";

interface MultipartUploadProps {
  onUploadComplete: (results: UploadResult[]) => void;
  onUploadProgress: (progress: number) => void;
  onUploadStart: () => void;
  onUploadError: (error: string) => void;
  isAuthenticated: boolean;
}

interface UploadResult {
  fileName: string;
  success: boolean;
  error?: string;
}

interface PartUpload {
  partNumber: number;
  etag: string;
}

const API_BASE_URL = process.env.REACT_APP_API_URL || "http://localhost:8080";
const CHUNK_SIZE = 5 * 1024 * 1024; // 5MB chunks
const MAX_FILE_SIZE = 100 * 1024 * 1024; // 100MB
const ALLOWED_TYPES = [
  "image/",
  "text/",
  "application/pdf",
  "application/msword",
];

const MultipartUpload: React.FC<MultipartUploadProps> = ({
  onUploadComplete,
  onUploadProgress,
  onUploadStart,
  onUploadError,
  isAuthenticated,
}) => {
  const [isDragOver, setIsDragOver] = useState(false);
  const [isUploading, setIsUploading] = useState(false);

  const validateFile = (file: File): string | null => {
    if (file.size > MAX_FILE_SIZE) {
      return `File ${file.name} is too large (max 100MB)`;
    }

    if (!ALLOWED_TYPES.some((type) => file.type.startsWith(type))) {
      return `File type ${file.type} is not allowed`;
    }

    return null;
  };

  const uploadFileChunk = async (
    presignedUrl: string,
    chunk: Blob
  ): Promise<string> => {
    const response = await fetch(presignedUrl, {
      method: "PUT",
      body: chunk,
      headers: {
        "Content-Type": "application/octet-stream",
      },
    });

    if (!response.ok) {
      throw new Error(`Failed to upload chunk: ${response.statusText}`);
    }

    const etag = response.headers.get("ETag");
    if (!etag) {
      throw new Error("No ETag received from upload");
    }

    return etag.replace(/"/g, ""); // Remove quotes from ETag
  };

  const uploadSingleFile = useCallback(
    async (file: File): Promise<UploadResult> => {
      try {
        // Step 1: Initiate multipart upload
        const initiateResponse = await fetch(
          `${API_BASE_URL}/upload/initiate`,
          {
            method: "POST",
            credentials: "include",
            headers: {
              Authorization: `Bearer ${localStorage.getItem("auth_token")}`,
              "Content-Type": "application/json",
            },
            body: JSON.stringify({
              filename: file.name,
            }),
          }
        );

        if (!initiateResponse.ok) {
          throw new Error("Failed to initiate multipart upload");
        }

        const { upload_id } = await initiateResponse.json();

        // Step 2: Upload file in chunks
        const chunks = Math.ceil(file.size / CHUNK_SIZE);
        const uploadedParts: PartUpload[] = [];

        for (let i = 0; i < chunks; i++) {
          const start = i * CHUNK_SIZE;
          const end = Math.min(start + CHUNK_SIZE, file.size);
          const chunk = file.slice(start, end);
          const partNumber = i + 1;

          // Get presigned URL for this part
          const presignedResponse = await fetch(
            `${API_BASE_URL}/upload/presigned-url`,
            {
              method: "POST",
              credentials: "include",
              headers: {
                "Content-Type": "application/json",
                Authorization: `Bearer ${localStorage.getItem("auth_token")}`,
              },
              body: JSON.stringify({
                filename: file.name,
                upload_id,
                part_number: partNumber,
                expires_in_secs: 3600,
              }),
            }
          );

          if (!presignedResponse.ok) {
            throw new Error(
              `Failed to get presigned URL for part ${partNumber}`
            );
          }

          const { presigned_url } = await presignedResponse.json();

          // Upload the chunk
          const etag = await uploadFileChunk(presigned_url, chunk);
          uploadedParts.push({ partNumber, etag });

          // Update progress
          const progress = ((i + 1) / chunks) * 100;
          onUploadProgress(progress);
        }

        // Step 3: Complete multipart upload
        const completeResponse = await fetch(
          `${API_BASE_URL}/upload/complete`,
          {
            method: "POST",
            credentials: "include",
            headers: {
              "Content-Type": "application/json",
              Authorization: `Bearer ${localStorage.getItem("auth_token")}`,
            },
            body: JSON.stringify({
              filename: file.name,
              upload_id,
              parts: uploadedParts.map((part) => [part.partNumber, part.etag]),
            }),
          }
        );

        if (!completeResponse.ok) {
          throw new Error("Failed to complete multipart upload");
        }

        return {
          fileName: file.name,
          success: true,
        };
      } catch (error) {
        return {
          fileName: file.name,
          success: false,
          error: error instanceof Error ? error.message : "Unknown error",
        };
      }
    },
    []
  );

  const handleFiles = useCallback(
    async (files: FileList | File[]) => {
      if (!isAuthenticated) {
        onUploadError("Please log in to upload files");
        return;
      }

      const fileArray = Array.from(files);

      // Validate all files first
      for (const file of fileArray) {
        const validationError = validateFile(file);
        if (validationError) {
          onUploadError(validationError);
          return;
        }
      }

      setIsUploading(true);
      onUploadStart();

      try {
        const results: UploadResult[] = [];

        for (let i = 0; i < fileArray.length; i++) {
          const file = fileArray[i];
          const result = await uploadSingleFile(file);
          results.push(result);

          // Update overall progress
          const overallProgress = ((i + 1) / fileArray.length) * 100;
          onUploadProgress(overallProgress);
        }

        onUploadComplete(results);
      } catch (error) {
        onUploadError(error instanceof Error ? error.message : "Upload failed");
      } finally {
        setIsUploading(false);
        onUploadProgress(0);
      }
    },
    [
      isAuthenticated,
      onUploadComplete,
      onUploadError,
      onUploadProgress,
      onUploadStart,
      uploadSingleFile,
    ]
  );

  const handleDrop = useCallback(
    (e: React.DragEvent<HTMLDivElement>) => {
      e.preventDefault();
      setIsDragOver(false);

      const files = e.dataTransfer.files;
      if (files.length > 0) {
        handleFiles(files);
      }
    },
    [handleFiles]
  );

  const handleDragOver = useCallback((e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    setIsDragOver(true);
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    setIsDragOver(false);
  }, []);

  const handleFileInput = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const files = e.target.files;
      if (files && files.length > 0) {
        handleFiles(files);
      }
      // Reset the input value so the same file can be selected again
      e.target.value = "";
    },
    [handleFiles]
  );

  return (
    <div className="w-full">
      <div
        className={`
          border-2 border-dashed rounded-lg p-8 text-center transition-colors
          ${
            isDragOver
              ? "border-blue-400 bg-blue-50"
              : "border-gray-300 hover:border-gray-400"
          }
          ${isUploading ? "opacity-50 pointer-events-none" : ""}
        `}
        onDrop={handleDrop}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
      >
        <div className="space-y-4">
          <div className="text-gray-600">
            <svg
              className="mx-auto h-12 w-12 text-gray-400"
              stroke="currentColor"
              fill="none"
              viewBox="0 0 48 48"
            >
              <path
                d="M28 8H12a4 4 0 00-4 4v20m32-12v8m0 0v8a4 4 0 01-4 4H12a4 4 0 01-4-4v-4m32-4l-3.172-3.172a4 4 0 00-5.656 0L28 28M8 32l9.172-9.172a4 4 0 015.656 0L28 28m0 0l4 4m4-24h8m-4-4v8m-12 4h.02"
                strokeWidth={2}
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
          </div>

          <div>
            <p className="text-lg font-medium text-gray-900">
              {isUploading ? "Uploading files..." : "Drop files here"}
            </p>
            <p className="text-sm text-gray-500">
              or{" "}
              <label className="text-blue-600 hover:text-blue-500 cursor-pointer">
                browse to choose files
                <input
                  type="file"
                  multiple
                  className="hidden"
                  onChange={handleFileInput}
                  disabled={isUploading}
                />
              </label>
            </p>
          </div>

          <div className="text-xs text-gray-400">
            <p>Supported: Images, Text files, PDF, Word documents</p>
            <p>Maximum file size: 100MB</p>
          </div>
        </div>
      </div>
    </div>
  );
};

export default MultipartUpload;
