"use client";

import { useEffect, useState } from "react";
import { Card } from "@/components/ui/card";
import { useParams, useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";

interface AlbumImage {
  id: string;
  album_id: string;
  file_name: string;
}

interface Album {
  id: string;
  name: string;
  description: string;
  date?: string;
  number_of_images: number;
}

const AlbumPage = () => {
  const { id } = useParams(); // Get album ID from URL
  const router = useRouter(); // For navigation

  const [album, setAlbum] = useState<Album | null>(null);
  const [images, setImages] = useState<AlbumImage[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isSlideshowActive, setIsSlideshowActive] = useState(false);
  const [currentSlide, setCurrentSlide] = useState(0);

  useEffect(() => {
    if (!id) return; // Don't fetch until we have an ID

    const fetchAlbumAndImages = async () => {
      try {
        console.log("Fetching images for album ID:", id);
        const response = await fetch(`http://localhost:8080/api/albums/${id}`);
        if (!response.ok) {
          throw new Error("Failed to fetch album images");
        }

        const data: { album: Album; images: AlbumImage[] } = await response.json();
        console.log("Fetched images:", data);

        setAlbum(data.album);
        setImages(data.images);
      } catch (err: any) {
        console.error("Error fetching images:", err.message);
        setError(err.message);
      } finally {
        setLoading(false);
      }
    };

    fetchAlbumAndImages();
  }, [id]);

  // Slideshow logic
  useEffect(() => {
    let slideshowInterval: NodeJS.Timeout | null = null;

    if (isSlideshowActive) {
      slideshowInterval = setInterval(() => {
        setCurrentSlide((prev) => (prev + 1) % images.length);
      }, 3000); // Change slide every 3 seconds
    }

    return () => {
      if (slideshowInterval) clearInterval(slideshowInterval);
    };
  }, [isSlideshowActive, images.length]);

  if (loading) {
    return <p className="text-center">Loading images...</p>;
  }

  if (error) {
    return <p className="text-center text-red-500">Error: {error}</p>;
  }

  if (!album) {
    return <p className="text-center text-red-500">Album not found.</p>;
  }

  return (
    <div className="container mx-auto p-6">
      {/* Header Section with Back Button, Album Name, and Slideshow Button */}
      <div className="flex justify-between items-center mb-8">
        {/* Back Button */}
        <Button variant="outline" onClick={() => router.push("/")} className="text-sm">
          ← Back to Home
        </Button>

        {/* Album Title */}
        <div className="text-center">
          <h1 className="text-3xl font-bold">{album.name}</h1>
          <p className="text-sm text-gray-500 mt-1">{album.date}</p>
        </div>

        {/* Slideshow Button */}
        <Button
          variant="outline"
          onClick={() => setIsSlideshowActive((prev) => !prev)}
          className="text-sm"
        >
          {isSlideshowActive ? "Stop Slideshow" : "Start Slideshow"}
        </Button>
      </div>

      {/* Album Description */}
      <p className="text-md text-gray-700 text-center mb-8">{album.description}</p>

      {/* Slideshow Mode */}
      {isSlideshowActive ? (
        <div className="flex justify-center">
          <div className="relative aspect-[4/3] w-full max-w-3xl">
            <img
              src={`http://localhost:8080/uploads/${encodeURIComponent(album.name)}/${encodeURIComponent(
                images[currentSlide].file_name
              )}`}
              alt={`Slide ${currentSlide + 1}`}
              className="absolute inset-0 w-full h-full object-cover rounded-lg"
            />
          </div>
        </div>
      ) : (
        // Normal Grid Mode
        <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-6">
          {images.map((image) => (
            <Card key={image.id}>
              <div className="relative aspect-[4/3]">
                <img
                  src={`http://localhost:8080/uploads/${encodeURIComponent(album.name)}/${encodeURIComponent(
                    image.file_name
                  )}`}
                  alt={`Image ${image.id}`}
                  className="absolute inset-0 w-full h-full object-cover rounded-lg"
                />
              </div>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
};

export default AlbumPage;
