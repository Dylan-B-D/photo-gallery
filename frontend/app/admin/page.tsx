"use client";

import { useCallback, useEffect, useState } from "react";
import { PlusCircle, Upload, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { useRouter } from "next/navigation";
import { getToken, isAuthenticated, removeToken } from "@/utils/auth";

// Interface for our file data
interface FileWithPreview {
  file: File;
  previewUrl: string;
  loading: boolean;
}

const CreateAlbumDialog = () => {
  const [isOpen, setIsOpen] = useState(false);
  const [albumData, setAlbumData] = useState({
    name: "",
    description: "",
    date: new Date().toISOString().split("T")[0],
  });
  const [selectedFiles, setSelectedFiles] = useState<FileWithPreview[]>([]);
  const [isUploading, setIsUploading] = useState(false);

  // Clean up object URLs when component unmounts or files are removed
  useEffect(() => {
    return () => {
      selectedFiles.forEach(file => URL.revokeObjectURL(file.previewUrl));
    };
  }, []);

  // Function to create optimized preview
  const createImagePreview = async (file: File): Promise<string> => {
    return new Promise((resolve) => {
      const canvas = document.createElement('canvas');
      const ctx = canvas.getContext('2d');
      const img = new Image();
      
      img.onload = () => {
        // Target dimensions for preview (thumbnail size)
        const maxWidth = 200;
        const maxHeight = 200;
        
        let width = img.width;
        let height = img.height;
        
        // Calculate new dimensions maintaining aspect ratio
        if (width > height) {
          if (width > maxWidth) {
            height = Math.round((height * maxWidth) / width);
            width = maxWidth;
          }
        } else {
          if (height > maxHeight) {
            width = Math.round((width * maxHeight) / height);
            height = maxHeight;
          }
        }
        
        canvas.width = width;
        canvas.height = height;
        
        ctx?.drawImage(img, 0, 0, width, height);
        
        // Convert to JPEG with reduced quality
        const previewUrl = canvas.toDataURL('image/jpeg', 0.5);
        URL.revokeObjectURL(img.src); // Clean up the temporary object URL
        resolve(previewUrl);
      };
      
      img.src = URL.createObjectURL(file);
    });
  };

  const handleFileSelect = async (e: React.ChangeEvent<HTMLInputElement>) => {
    if (!e.target.files) return;
    
    const newFiles = Array.from(e.target.files).map(file => ({
      file,
      previewUrl: '',
      loading: true
    }));
    
    setSelectedFiles(prev => [...prev, ...newFiles]);
    
    // Create optimized previews
    for (let i = 0; i < newFiles.length; i++) {
      const previewUrl = await createImagePreview(newFiles[i].file);
      setSelectedFiles(prev => prev.map((fileData, index) => {
        if (fileData === newFiles[i]) {
          return { ...fileData, previewUrl, loading: false };
        }
        return fileData;
      }));
    }
  };

  const removeFile = (indexToRemove: number) => {
    setSelectedFiles(prev => {
      const newFiles = prev.filter((_, index) => index !== indexToRemove);
      // Clean up the removed file's preview URL
      URL.revokeObjectURL(prev[indexToRemove].previewUrl);
      return newFiles;
    });
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsUploading(true);

    try {
      const formData = new FormData();
      formData.append("name", albumData.name);
      formData.append("description", albumData.description);
      formData.append("date", albumData.date);

      // Only append the files that are still in the selectedFiles array
      selectedFiles.forEach(fileData => {
        formData.append("images", fileData.file);
      });

      const token = getToken();
      const response = await fetch("http://localhost:8080/api/albums", {
        method: "POST",
        headers: {
          Authorization: `Bearer ${token}`,
        },
        body: formData,
      });

      if (!response.ok) {
        throw new Error("Failed to create album");
      }

      // Clean up all preview URLs
      selectedFiles.forEach(fileData => URL.revokeObjectURL(fileData.previewUrl));
      
      // Reset form
      setAlbumData({
        name: "",
        description: "",
        date: new Date().toISOString().split("T")[0],
      });
      setSelectedFiles([]);
      setIsOpen(false);
    } catch (error) {
      console.error("Error creating album:", error);
    } finally {
      setIsUploading(false);
    }
  };

  const isFormValid = useCallback(() => {
    return (
      albumData.name.trim() !== "" &&
      albumData.description.trim() !== "" &&
      albumData.date !== "" &&
      selectedFiles.length > 0
    );
  }, [albumData, selectedFiles]);

  const removeAllFiles = () => {
    // Clean up all preview URLs before removing
    selectedFiles.forEach(fileData => URL.revokeObjectURL(fileData.previewUrl));
    setSelectedFiles([]);
  };

  return (
    <Dialog open={isOpen} onOpenChange={setIsOpen}>
      <DialogTrigger asChild>
        <Button className="flex items-center gap-2">
          <PlusCircle className="w-4 h-4" />
          Create New Album
        </Button>
      </DialogTrigger>
      <DialogContent className="w-[calc(100vw-8px)] h-[calc(100vh-8px)] max-w-none p-0 flex flex-col">
        <DialogHeader className="p-4 border-b shrink-0">
          <DialogTitle>Create New Album</DialogTitle>
        </DialogHeader>
        <form
          onSubmit={handleSubmit}
          className="flex-1 overflow-hidden flex flex-col"
        >
          {/* Rest of the form fields remain the same */}
          <div className="flex-1 overflow-y-auto p-4">
            <div className="space-y-4 mb-4">
              <div>
                <Input
                  required
                  value={albumData.name}
                  onChange={(e) =>
                    setAlbumData((prev) => ({ ...prev, name: e.target.value }))
                  }
                  placeholder="Album Name"
                />
              </div>

              <div>
                <Textarea
                  required
                  value={albumData.description}
                  onChange={(e) =>
                    setAlbumData((prev) => ({
                      ...prev,
                      description: e.target.value,
                    }))
                  }
                  placeholder="Album Description"
                  rows={2}
                />
              </div>

              <div className="flex gap-2 items-center">
                <Input
                  type="date"
                  required
                  value={albumData.date}
                  onChange={(e) =>
                    setAlbumData((prev) => ({ ...prev, date: e.target.value }))
                  }
                  className="flex-1"
                />
                <label className="flex-1 h-10 cursor-pointer flex items-center justify-center rounded-md border border-input bg-background hover:bg-accent hover:text-accent-foreground">
                  <Upload className="h-4 w-4 mr-2" />
                  <span className="text-sm">Upload Images</span>
                  <input
                    type="file"
                    className="hidden"
                    multiple
                    accept="image/*"
                    onChange={handleFileSelect}
                  />
                </label>
              </div>
            </div>

            <Card>
              <CardHeader className="py-2">
                <CardTitle className="text-sm">Images</CardTitle>
              </CardHeader>
              <CardContent>
                {selectedFiles.length > 0 ? (
                  <div className="max-h-[calc(100vh-450px)] overflow-y-auto">
                    <div
                      className="grid gap-2"
                      style={{
                        gridTemplateColumns:
                          "repeat(auto-fit, minmax(80px, 1fr))",
                      }}
                    >
                      {selectedFiles.map((fileData, index) => (
                        <div key={index} className="relative group">
                          <div className="w-20 h-20 rounded-md border bg-muted flex items-center justify-center overflow-hidden relative">
                            {fileData.loading ? (
                              <div className="animate-pulse bg-gray-200 w-full h-full" />
                            ) : (
                              <img
                                src={fileData.previewUrl}
                                alt={`Preview ${index + 1}`}
                                className="object-cover w-full h-full rounded-md"
                              />
                            )}
                            <button
                              type="button"
                              onClick={() => removeFile(index)}
                              className="absolute top-1 right-1 bg-destructive text-destructive-foreground rounded-full p-0.5 opacity-0 group-hover:opacity-100 transition-opacity"
                            >
                              <X className="w-3 h-3" />
                            </button>
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                ) : (
                  <p className="text-sm text-muted-foreground text-center py-4">
                    No images uploaded yet
                  </p>
                )}
              </CardContent>
            </Card>
          </div>
          <div className="border-t p-4 flex justify-between items-center gap-4 shrink-0">
            <Button
              type="button"
              variant="outline"
              onClick={removeAllFiles}
              disabled={selectedFiles.length === 0}
              className="text-sm"
            >
              Remove All Images
            </Button>

            <div className="flex gap-4">
              <Button
                type="button"
                variant="outline"
                onClick={() => setIsOpen(false)}
              >
                Cancel
              </Button>
              <Button
                type="submit"
                disabled={isUploading || !isFormValid()}
                onClick={handleSubmit}
              >
                {isUploading ? "Creating Album..." : "Create Album"}
              </Button>
            </div>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  );
};

export default function AdminPanel() {
  const router = useRouter();
  const [isLoading, setIsLoading] = useState(true);
  const [authStatus, setAuthStatus] = useState(false);

  useEffect(() => {
    const checkAuth = async () => {
      const isAuth = await isAuthenticated();
      if (!isAuth) {
        setAuthStatus(false);
      } else {
        setAuthStatus(true);
      }
      setIsLoading(false);
    };
    checkAuth();
  }, [router]);

  const handleLogout = () => {
    removeToken();
    router.push("/");
  };

  if (isLoading) {
    return <div>Loading...</div>;
  }

  if (!authStatus) {
    return (
      <div className="container mx-auto p-6 text-center flex items-center justify-center min-h-screen">
        <div>
          <h1 className="text-3xl font-bold">Access Denied</h1>
          <p>You need to be logged in to view this page.</p>
          <div className="mt-4">
            <Button onClick={() => router.push("/")} className="mr-2">
              Go to Home
            </Button>
            <Button onClick={() => router.push("/login")} variant="secondary">
              Go to Login
            </Button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="container mx-auto p-6">
      <div className="flex justify-between items-center mb-8">
        <div>
          <h1 className="text-3xl font-bold">Admin Panel</h1>
          <p className="text-gray-600">Manage your photo albums</p>
        </div>
        <div className="flex gap-4">
          {/* Create album dialog */}
          <CreateAlbumDialog />
          {/* Logout button */}
          <Button onClick={handleLogout} variant="destructive">
            Logout
          </Button>
        </div>
      </div>
      {/*
        You can list existing albums here,
        or create a separate table component for it.
      */}
    </div>
  );
}
