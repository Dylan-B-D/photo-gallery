"use client";

import Image from "next/image";
import { useState, useEffect, useMemo, useCallback } from "react";
import { useRouter } from "next/navigation";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { PlusCircle, X } from "lucide-react";
import { getToken, isAuthenticated, removeToken } from "@/utils/auth";

// Interface for album data
interface Album {
  id: string;
  name: string;
  description: string;
  date?: string;
  number_of_images: number;
  thumbnail?: string;
}

// Interface for file data with preview
interface FileWithPreview {
  file: File;
  previewUrl: string;
  loading: boolean;
}

// Interface for album image data
interface AlbumImage {
  id: string;
  file_name: string;
  previewUrl?: string;
}

const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL;

// CreateAlbumDialog Component
const CreateAlbumDialog = () => {
  const [isOpen, setIsOpen] = useState(false);
  const [albumData, setAlbumData] = useState({
    name: "",
    description: "",
    date: new Date().toISOString().split("T")[0],
  });
  const [selectedFiles, setSelectedFiles] = useState<FileWithPreview[]>([]);
  const [isUploading, setIsUploading] = useState(false);

  useEffect(() => {
    return () => {
      selectedFiles.forEach((file) => URL.revokeObjectURL(file.previewUrl));
    };
  }, [selectedFiles]);

  const createImagePreview = async (file: File): Promise<string> => {
    return new Promise((resolve) => {
      const canvas = document.createElement("canvas");
      const ctx = canvas.getContext("2d");
      const img = document.createElement("img");

      img.onload = () => {
        const maxWidth = 200;
        const maxHeight = 200;
        let width = img.width;
        let height = img.height;

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
        const previewUrl = canvas.toDataURL("image/jpeg", 0.5);
        URL.revokeObjectURL(img.src);
        resolve(previewUrl);
      };

      img.src = URL.createObjectURL(file);
    });
  };

  const handleFileSelect = async (e: React.ChangeEvent<HTMLInputElement>) => {
    if (!e.target.files) return;

    const newFiles = Array.from(e.target.files).map((file) => ({
      file,
      previewUrl: "",
      loading: true,
    }));

    setSelectedFiles((prev) => [...prev, ...newFiles]);

    for (let i = 0; i < newFiles.length; i++) {
      const previewUrl = await createImagePreview(newFiles[i].file);
      setSelectedFiles((prev) =>
        prev.map((fileData) => {
          if (fileData === newFiles[i]) {
            return { ...fileData, previewUrl, loading: false };
          }
          return fileData;
        })
      );
    }
  };

  const removeFile = (indexToRemove: number) => {
    setSelectedFiles((prev) => {
      const newFiles = prev.filter((_, index) => index !== indexToRemove);
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

      selectedFiles.forEach((fileData) => {
        formData.append("images", fileData.file);
      });

      const token = getToken();
      const response = await fetch(`${API_BASE_URL}/api/albums`, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${token}`,
        },
        body: formData,
      });

      if (!response.ok) {
        throw new Error("Failed to create album");
      }

      selectedFiles.forEach((fileData) =>
        URL.revokeObjectURL(fileData.previewUrl)
      );

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

  const isFormValid = () => {
    return (
      albumData.name.trim() !== "" &&
      albumData.description.trim() !== "" &&
      albumData.date !== "" &&
      selectedFiles.length > 0
    );
  };

  const removeAllFiles = () => {
    selectedFiles.forEach((fileData) =>
      URL.revokeObjectURL(fileData.previewUrl)
    );
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
          <div className="flex-1 overflow-y-auto p-4">
            <div className="space-y-4 mb-4">
              <Input
                required
                value={albumData.name}
                onChange={(e) =>
                  setAlbumData((prev) => ({ ...prev, name: e.target.value }))
                }
                placeholder="Album Name"
              />
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
                            <Image
                              src={fileData.previewUrl}
                              alt={`Preview ${index + 1}`}
                              fill
                              style={{ objectFit: "cover" }}
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
              <Button type="submit" disabled={isUploading || !isFormValid()}>
                {isUploading ? "Creating Album..." : "Create Album"}
              </Button>
            </div>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  );
};

// EditAlbumDialog Component
const EditAlbumDialog = ({
  album,
  isOpen,
  onOpenChange,
  onAlbumUpdate,
}: {
  album: Album | null;
  isOpen: boolean;
  onOpenChange: (isOpen: boolean) => void;
  onAlbumUpdate: () => void;
}) => {
  const [albumData, setAlbumData] = useState({
    name: album?.name || "",
    description: album?.description || "",
    date: album?.date || new Date().toISOString().split("T")[0],
  });
  const [selectedFiles, setSelectedFiles] = useState<FileWithPreview[]>([]);
  const [existingImages, setExistingImages] = useState<AlbumImage[]>([]);
  const [isUploading, setIsUploading] = useState(false);
  const [imagesToDelete, setImagesToDelete] = useState<Set<string>>(new Set());

  const fetchAlbumImages = useCallback(
    async (albumId: string) => {
      try {
        const token = getToken();
        const response = await fetch(
          `${API_BASE_URL}/api/albums/${albumId}`,
          {
            headers: {
              Authorization: `Bearer ${token}`,
            },
          }
        );
        if (!response.ok) throw new Error("Failed to fetch album images");
        const data = await response.json();

        // Use optional chaining and default to an empty string if album is null
        const albumName = album?.name || "";

        const downscaledImages = await Promise.all(
          data.images.map(async (image: AlbumImage) => ({
            ...image,
            previewUrl: await downscaleExistingImage(
              `${API_BASE_URL}/uploads/${encodeURIComponent(
                albumName
              )}/${encodeURIComponent(image.file_name)}`
            ),
          }))
        );

        setExistingImages(downscaledImages);
      } catch (error) {
        console.error("Error fetching album images:", error);
      }
    },
    [album]
  );

  useEffect(() => {
    if (album) {
      setAlbumData({
        name: album.name,
        description: album.description,
        date: album.date || new Date().toISOString().split("T")[0],
      });
      fetchAlbumImages(album.id);
    }
    return () => {
      selectedFiles.forEach((file) => URL.revokeObjectURL(file.previewUrl));
    };
  }, [album, fetchAlbumImages, selectedFiles]);

  const downscaleExistingImage = async (url: string): Promise<string> => {
    return new Promise((resolve) => {
      const img = document.createElement("img");
      img.crossOrigin = "anonymous"; // Ensure cross-origin is handled correctly

      img.onload = () => {
        const canvas = document.createElement("canvas");
        const ctx = canvas.getContext("2d");
        const maxSize = 100; // Use a smaller max size for thumbnails
        let width = img.width;
        let height = img.height;

        if (width > height && width > maxSize) {
          height = Math.round((height * maxSize) / width);
          width = maxSize;
        } else if (height > maxSize) {
          width = Math.round((width * maxSize) / height);
          height = maxSize;
        }

        canvas.width = width;
        canvas.height = height;
        ctx?.drawImage(img, 0, 0, width, height);
        resolve(canvas.toDataURL("image/jpeg", 0.5)); // Downscale and reduce quality
      };

      img.onerror = () => resolve(url); // Fallback to original URL if an error occurs

      img.src = url;
    });
  };

  // Optimized image preview creation
  const createImagePreview = useMemo(() => {
    return async (file: File): Promise<string> => {
      return new Promise((resolve) => {
        // Use createImageBitmap for better performance
        createImageBitmap(file)
          .then((bitmap) => {
            const canvas = document.createElement("canvas");
            const ctx = canvas.getContext("2d");
            const maxSize = 100; // Reduced preview size
            let width = bitmap.width;
            let height = bitmap.height;

            if (width > height && width > maxSize) {
              height = Math.round((height * maxSize) / width);
              width = maxSize;
            } else if (height > maxSize) {
              width = Math.round((width * maxSize) / height);
              height = maxSize;
            }

            canvas.width = width;
            canvas.height = height;
            ctx?.drawImage(bitmap, 0, 0, width, height);
            bitmap.close(); // Clean up the bitmap
            resolve(canvas.toDataURL("image/jpeg", 0.3)); // Reduced quality
          })
          .catch(() => {
            // Fallback for unsupported formats
            const reader = new FileReader();
            reader.onload = (e) => resolve(e.target?.result as string);
            reader.readAsDataURL(file);
          });
      });
    };
  }, []);

  const toggleImageDeletion = (imageId: string) => {
    setImagesToDelete((prev) => {
      const updated = new Set(prev);
      if (updated.has(imageId)) {
        updated.delete(imageId); // Unmark if already marked
      } else {
        updated.add(imageId); // Mark for deletion
      }
      return updated;
    });
  };

  const handleFileSelect = async (e: React.ChangeEvent<HTMLInputElement>) => {
    if (!e.target.files) return;

    const newFiles = Array.from(e.target.files).map((file) => ({
      file,
      previewUrl: "",
      loading: true,
    }));

    setSelectedFiles((prev) => [...prev, ...newFiles]);

    for (let i = 0; i < newFiles.length; i++) {
      const previewUrl = await createImagePreview(newFiles[i].file);
      setSelectedFiles((prev) =>
        prev.map((fileData) => {
          if (fileData === newFiles[i]) {
            return { ...fileData, previewUrl, loading: false };
          }
          return fileData;
        })
      );
    }
  };

  const removeFile = (indexToRemove: number) => {
    setSelectedFiles((prev) => {
      const newFiles = prev.filter((_, index) => index !== indexToRemove);
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

      selectedFiles.forEach((fileData) => {
        formData.append("images", fileData.file);
      });

      if (imagesToDelete.size > 0) {
        formData.append(
          "imagesToDelete",
          JSON.stringify(Array.from(imagesToDelete))
        );
      }

      const token = getToken();
      const response = await fetch(
        `${API_BASE_URL}/api/albums/${album?.id}`,
        {
          method: "PUT",
          headers: {
            Authorization: `Bearer ${token}`,
          },
          body: formData,
        }
      );

      if (!response.ok) throw new Error("Failed to update album");

      selectedFiles.forEach((fileData) =>
        URL.revokeObjectURL(fileData.previewUrl)
      );
      onAlbumUpdate();
      onOpenChange(false);
    } catch (error) {
      console.error("Error updating album:", error);
    } finally {
      setIsUploading(false);
    }
  };

  const isFormValid = () => {
    return (
      albumData.name.trim() !== "" &&
      albumData.description.trim() !== "" &&
      albumData.date !== ""
    );
  };

  const removeAllFiles = () => {
    selectedFiles.forEach((fileData) =>
      URL.revokeObjectURL(fileData.previewUrl)
    );
    setSelectedFiles([]);
  };

  return (
    <Dialog open={isOpen} onOpenChange={onOpenChange}>
      <DialogContent className="w-[calc(100vw-8px)] h-[calc(100vh-8px)] max-w-none p-0 flex flex-col">
        <DialogHeader className="p-4 border-b shrink-0">
          <DialogTitle>Edit Album</DialogTitle>
        </DialogHeader>
        <form
          onSubmit={handleSubmit}
          className="flex-1 overflow-hidden flex flex-col"
        >
          <div className="flex-1 overflow-y-auto p-4">
            <div className="space-y-4 mb-4">
              <Input
                required
                value={albumData.name}
                onChange={(e) =>
                  setAlbumData((prev) => ({ ...prev, name: e.target.value }))
                }
                placeholder="Album Name"
              />
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
                <CardTitle className="text-sm">Existing Images</CardTitle>
              </CardHeader>
              <CardContent>
                {existingImages.length > 0 ? (
                  <div
                    className="grid gap-2"
                    style={{
                      gridTemplateColumns:
                        "repeat(auto-fit, minmax(80px, 1fr))",
                    }}
                  >
                    {existingImages.map((image) => (
                      <div key={image.id} className="relative group">
                        <div
                          className={`w-20 h-20 rounded-md border bg-muted flex items-center justify-center overflow-hidden relative ${
                            imagesToDelete.has(image.id)
                              ? "opacity-50 border-red-500"
                              : ""
                          }`}
                          onClick={() => toggleImageDeletion(image.id)} // Toggle on click
                        >
                          <Image
                            src={
                              image.previewUrl ||
                              `${API_BASE_URL}/uploads/${encodeURIComponent(
                                album?.name || ""
                              )}/${encodeURIComponent(image.file_name)}`
                            }
                            alt={`Image ${image.id}`}
                            fill
                            style={{ objectFit: "cover" }}
                            className="object-cover w-full h-full rounded-md"
                          />
                          <button
                            type="button"
                            onClick={(e) => {
                              e.stopPropagation(); // Prevent toggle when clicking the button
                              toggleImageDeletion(image.id);
                            }}
                            className="absolute top-1 right-1 bg-destructive text-destructive-foreground rounded-full p-0.5 opacity-0 group-hover:opacity-100 transition-opacity"
                          >
                            <X className="w-3 h-3" />
                          </button>
                        </div>
                      </div>
                    ))}
                  </div>
                ) : (
                  <p className="text-sm text-muted-foreground text-center py-4">
                    Loading/No Existing Images
                  </p>
                )}
              </CardContent>
            </Card>

            <Card className="mt-4">
              <CardHeader className="py-2">
                <CardTitle className="text-sm">New Images</CardTitle>
              </CardHeader>
              <CardContent>
                {selectedFiles.length > 0 ? (
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
                            <Image
                              src={fileData.previewUrl}
                              alt={`Preview ${index + 1}`}
                              fill
                              style={{ objectFit: "cover" }}
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
                ) : (
                  <p className="text-sm text-muted-foreground text-center py-4">
                    No new images uploaded
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
              Remove All New Images
            </Button>
            <div className="flex gap-4">
              <Button
                type="button"
                variant="outline"
                onClick={() => onOpenChange(false)}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={isUploading || !isFormValid()}>
                {isUploading ? "Updating Album..." : "Update Album"}
              </Button>
            </div>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  );
};

const AlbumCard = ({
  album,
  onEdit,
  onDelete,
}: {
  album: Album;
  onEdit: (album: Album) => void;
  onDelete: (id: string) => void;
}) => {
  const [firstImage, setFirstImage] = useState<string | null>(null);

  useEffect(() => {
    const fetchFirstImage = async () => {
      try {
        const response = await fetch(
          `${API_BASE_URL}/api/albums/${album.id}`
        );
        if (!response.ok) throw new Error("Failed to fetch album images");
        const data = await response.json();
        if (data.images && data.images.length > 0) {
          setFirstImage(
            `${API_BASE_URL}/uploads/${encodeURIComponent(
              album.name
            )}/${encodeURIComponent(data.images[0].file_name)}`
          );
        }
      } catch (error) {
        console.error("Error fetching album images:", error);
      }
    };
    fetchFirstImage();
  }, [album]);

  return (
    <Card className="relative" style={{ maxWidth: "400px" }}>
      {firstImage && (
        <div className="relative aspect-[4/3]">
          <Image
            src={firstImage}
            alt={`Cover for ${album.name}`}
            fill
            style={{ objectFit: "cover" }}
            className="absolute inset-0 w-full h-full object-cover rounded-t-lg"
            loading="lazy"
            sizes="(max-width: 768px) 100vw, (max-width: 1200px) 50vw, 33vw"
          />
          <Badge className="absolute top-2 right-2">
            {album.number_of_images} images
          </Badge>
        </div>
      )}
      <CardHeader>
        <div className="flex justify-between items-center">
          <CardTitle>{album.name}</CardTitle>
          <p className="text-sm text-muted-foreground">{album.date}</p>
        </div>
      </CardHeader>
      <CardContent>
        <p className="line-clamp-2 mb-2">{album.description}</p>
        <div className="flex gap-2 mt-4">
          <Button variant="outline" onClick={() => onEdit(album)}>
            Edit
          </Button>
          <Button variant="destructive" onClick={() => onDelete(album.id)}>
            Delete
          </Button>
        </div>
      </CardContent>
    </Card>
  );
};

// AdminPanel Component
export default function AdminPanel() {
  const router = useRouter();
  const [isLoading, setIsLoading] = useState(true);
  const [getAuthStatus, setAuthStatus] = useState(false);
  const [albums, setAlbums] = useState<Album[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [isEditOpen, setIsEditOpen] = useState(false);
  const [editAlbum, setEditAlbum] = useState<Album | null>(null);

  const fetchAdminAlbums = async () => {
    try {
      const token = getToken();
      const response = await fetch(`${API_BASE_URL}/api/albums`, {
        method: "GET",
        headers: {
          Authorization: `Bearer ${token}`,
        },
      });

      if (!response.ok) {
        throw new Error("Failed to fetch albums");
      }

      const albumsData: Album[] = await response.json();
      setAlbums(albumsData);
    } catch (err) {
      if (err instanceof Error) {
        console.error("Error fetching albums:", err.message);
        setError(err.message);
      } else {
        console.error("Error fetching albums:", err);
        setError("An unknown error occurred");
      }
    }
  };

  useEffect(() => {
    const checkAuth = async () => {
      const isAuth = await isAuthenticated();
      if (!isAuth) {
        setAuthStatus(false);
        router.push("/login");
      } else {
        setAuthStatus(true);
        await fetchAdminAlbums();
      }
      setIsLoading(false);
    };
    checkAuth();
  }, [router]);

  if (isLoading) {
    return (
      <div
        style={{
          display: "flex",
          justifyContent: "center",
          alignItems: "center",
          height: "100vh",
        }}
      >
        <div className="spinner"></div>
      </div>
    );
  }

  if (getAuthStatus === false) {
    return null;
  }

  const handleLogout = () => {
    removeToken();
    router.push("/");
  };

  const openEditDialog = (album: Album) => {
    setEditAlbum(album);
    setIsEditOpen(true);
  };

  const handleDeleteAlbum = async (id: string) => {
    if (!confirm("Are you sure you want to delete this album?")) return;

    try {
      const token = getToken();
      const response = await fetch(`${API_BASE_URL}/api/albums/${id}`, {
        method: "DELETE",
        headers: {
          Authorization: `Bearer ${token}`,
        },
      });

      if (!response.ok) {
        throw new Error("Failed to delete album");
      }

      await fetchAdminAlbums();
    } catch (error) {
      console.error("Error deleting album:", error);
    }
  };

  return (
    <div className="container mx-auto p-6">
      <div className="flex justify-between items-center mb-8">
        <h1 className="text-3xl font-bold">Manage Albums</h1>
        <div className="flex gap-4">
          <CreateAlbumDialog />
          <Button onClick={handleLogout} variant="destructive">
            Logout
          </Button>
          <Button
            variant="outline"
            onClick={() => router.push("/")}
            className="text-sm"
          >
            ← Back to Home
          </Button>
        </div>
      </div>

      {error && <p className="text-red-500">Error: {error}</p>}

      {albums.length === 0 ? (
        <p>No albums found.</p>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-6">
          {albums.map((album) => (
            <AlbumCard
              key={album.id}
              album={album}
              onEdit={openEditDialog}
              onDelete={handleDeleteAlbum}
            />
          ))}
        </div>
      )}

      <EditAlbumDialog
        album={editAlbum}
        isOpen={isEditOpen}
        onOpenChange={setIsEditOpen}
        onAlbumUpdate={fetchAdminAlbums}
      />
    </div>
  );
}
