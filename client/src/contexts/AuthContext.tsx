"use client";

import React, {
  createContext,
  useContext,
  useState,
  useEffect,
  ReactNode,
  useCallback,
  useMemo,
} from "react";

interface AuthContextType {
  isAuthenticated: boolean;
  username: string;
  login: (email: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  checkAuthStatus: () => void;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

const API_BASE_URL = process.env.REACT_APP_API_URL || "http://localhost:8080";

interface AuthProviderProps {
  children: ReactNode;
}

export const AuthProvider: React.FC<AuthProviderProps> = ({ children }) => {
  const [isAuthenticated, setIsAuthenticated] = useState<boolean>(false);
  const [username, setUsername] = useState<string>("");
  const [isLoading, setIsLoading] = useState<boolean>(true);

  // Check if user is authenticated on app load
  const checkAuthStatus = useCallback(() => {
    // Check if auth token exists in localStorage
    const token = localStorage.getItem("auth_token");
    const storedUsername = localStorage.getItem("username");

    if (!token || !storedUsername) {
      setIsAuthenticated(false);
      setUsername("");
      setIsLoading(false);
    } else {
      setIsAuthenticated(true);
      setUsername(storedUsername);
      setIsLoading(false);
    }
  }, []);

  const login = useCallback(
    async (email: string, password: string): Promise<void> => {
      try {
        const response = await fetch(`${API_BASE_URL}/auth/login`, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({ email, password }),
        });

        if (response.ok) {
          const { token, username } = await response.json();
          setIsAuthenticated(true);
          setUsername(username);

          // Store token and username in localStorage for persistence
          localStorage.setItem("auth_token", token);
          localStorage.setItem("username", username);
        } else {
          const error = await response.json();
          throw new Error(error.message || "Login failed");
        }
      } catch (error) {
        console.error("Login failed:", error);
        throw error;
      }
    },
    []
  );

  const logout = useCallback(async (): Promise<void> => {
    // Simply clear local state and localStorage
    setIsAuthenticated(false);
    setUsername("");
    localStorage.removeItem("auth_token");
    localStorage.removeItem("username");
  }, []);

  // Check auth status on component mount
  useEffect(() => {
    checkAuthStatus();
  }, [checkAuthStatus]);

  const contextValue = useMemo(
    () => ({
      isAuthenticated,
      username,
      login,
      logout,
      checkAuthStatus,
    }),
    [isAuthenticated, username, login, logout, checkAuthStatus]
  );

  // Show loading state while checking authentication
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

  return (
    <AuthContext.Provider value={contextValue}>{children}</AuthContext.Provider>
  );
};

export const useAuth = (): AuthContextType => {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error("useAuth must be used within an AuthProvider");
  }
  return context;
};
