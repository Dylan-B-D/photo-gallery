"use client";

import { useEffect, useState, useRef } from "react";
import { Card } from "@/components/ui/card";
import { useParams, useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import {
  ArrowLeft,
  ArrowRight,
  Pause,
  Play,
  X,
  Download,
  Info,
  Maximize2,
  Minimize2,
} from "lucide-react";

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
  const { id } = useParams();
  const router = useRouter();

  const [album, setAlbum] = useState<Album | null>(null);
  const [images, setImages] = useState<AlbumImage[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isSlideshowActive, setIsSlideshowActive] = useState(false);
  const [currentSlide, setCurrentSlide] = useState(0);
  const [isPaused, setIsPaused] = useState(false);
  const [controlsVisible, setControlsVisible] = useState(true);
  const [transitionTime, setTransitionTime] = useState(3000); // Default transition time in milliseconds
  const [isViewingMode, setIsViewingMode] = useState(false); // Viewing mode flag
  const [showInfo, setShowInfo] = useState(false); // Info panel toggle

  const controlsTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  useEffect(() => {
    if (!id) return;

    const fetchAlbumAndImages = async () => {
      try {
        const response = await fetch(`http://localhost:8080/api/albums/${id}`);
        if (!response.ok) throw new Error("Failed to fetch album images");

        const data: { album: Album; images: AlbumImage[] } =
          await response.json();

        setAlbum(data.album);
        setImages(data.images);
      } catch (err: any) {
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

    if (isSlideshowActive && !isPaused) {
      slideshowInterval = setInterval(() => {
        setCurrentSlide((prev) => (prev + 1 < images.length ? prev + 1 : prev));
      }, transitionTime);
    }

    return () => {
      if (slideshowInterval) clearInterval(slideshowInterval);
    };
  }, [isSlideshowActive, isPaused, images.length, transitionTime]);

  // Keyboard controls for Viewing Mode
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (isViewingMode) {
        switch (e.key) {
          case "Escape":
            setIsViewingMode(false);
            break;
          case "ArrowLeft":
            setCurrentSlide((prev) => Math.max(prev - 1, 0));
            break;
          case "ArrowRight":
            setCurrentSlide((prev) =>
              prev + 1 < images.length ? prev + 1 : prev
            );
            break;
          case "f":
            toggleFullscreen();
            break;
          default:
            break;
        }
      } else if (isSlideshowActive) {
        switch (e.key) {
          case "Escape":
            setIsSlideshowActive(false);
            break;
          case " ":
            e.preventDefault();
            setIsPaused((prev) => !prev);
            break;
          case "ArrowLeft":
            setCurrentSlide((prev) => Math.max(prev - 1, 0));
            break;
          case "ArrowRight":
            setCurrentSlide((prev) =>
              prev + 1 < images.length ? prev + 1 : prev
            );
            break;
          case "f":
            toggleFullscreen();
            break;
          default:
            break;
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isViewingMode, isSlideshowActive, images.length]);

  const toggleFullscreen = () => {
    if (!document.fullscreenElement) {
      document.documentElement.requestFullscreen();
    } else {
      document.exitFullscreen();
    }
  };

  const handleImageClick = (index: number) => {
    setCurrentSlide(index);
    setIsViewingMode(true);
  };

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
      {isViewingMode ? (
        // Viewing Mode
        <div className="fixed inset-0 bg-black flex items-center justify-center">
          {/* Back Button and Count */}
          <div className="absolute top-4 left-4 flex items-center space-x-4">
            <Button
              variant="outline"
              onClick={() => {
                setIsViewingMode(false);
                if (document.fullscreenElement) {
                  document.exitFullscreen();
                }
              }}
              className="text-white"
            >
              <X />
            </Button>
            <span className="text-white text-lg">
              {currentSlide + 1}/{images.length}
            </span>
          </div>

          {/* Previous Button */}
          {currentSlide > 0 && (
            <Button
              variant="ghost"
              onClick={() => setCurrentSlide((prev) => Math.max(prev - 1, 0))}
              className="absolute left-4 text-white"
            >
              <ArrowLeft size={48} />
            </Button>
          )}

          {/* Image */}
          <img
            src={`http://localhost:8080/uploads/${encodeURIComponent(
              album.name
            )}/${encodeURIComponent(images[currentSlide].file_name)}`}
            alt={`Image ${currentSlide + 1}`}
            className="max-w-full max-h-full object-contain"
          />

          {/* Next Button */}
          {currentSlide < images.length - 1 && (
            <Button
              variant="ghost"
              onClick={() =>
                setCurrentSlide((prev) =>
                  prev + 1 < images.length ? prev + 1 : prev
                )
              }
              className="absolute right-4 text-white"
            >
              <ArrowRight size={48} />
            </Button>
          )}

          {/* Download and Info Buttons */}
          <div className="absolute bottom-4 right-4 flex items-center space-x-4">
            <Button
              variant="outline"
              onClick={async () => {
                const url = `http://localhost:8080/uploads/${encodeURIComponent(
                  album.name
                )}/${encodeURIComponent(images[currentSlide].file_name)}`;

                try {
                  // Fetch the file as a blob
                  const response = await fetch(url);
                  if (!response.ok) {
                    throw new Error(
                      `Failed to fetch image. Status: ${response.status}`
                    );
                  }

                  const blob = await response.blob();

                  // Create a temporary URL for the blob
                  const tempUrl = URL.createObjectURL(blob);

                  // Create an anchor element and trigger click
                  const link = document.createElement("a");
                  link.href = tempUrl;
                  link.download = images[currentSlide].file_name;
                  link.click();

                  // Clean up the temporary object URL
                  URL.revokeObjectURL(tempUrl);
                } catch (error) {
                  console.error("Download failed:", error);
                }
              }}
              className="text-white"
            >
              <Download />
            </Button>

            <Button
              variant="outline"
              onClick={() => setShowInfo((prev) => !prev)}
              className="text-white"
            >
              <Info />
            </Button>
          </div>

          {/* Info Panel */}
          {showInfo && (
            <div className="absolute bottom-16 right-4 bg-gray-800 bg-opacity-75 p-4 rounded-lg text-white">
              <h2 className="text-lg font-bold mb-2">Image Metadata</h2>
              <p>
                <strong>Camera Make:</strong> Coming Soon
              </p>
              <p>
                <strong>Camera Model:</strong> Coming Soon
              </p>
              <p>
                <strong>Lens Model:</strong> Coming Soon
              </p>
              <p>
                <strong>ISO:</strong> Coming Soon
              </p>
              <p>
                <strong>Aperture:</strong> Coming Soon
              </p>
              <p>
                <strong>Shutter Speed:</strong> Coming Soon
              </p>
              <p>
                <strong>Focal Length:</strong> Coming Soon
              </p>
            </div>
          )}
        </div>
      ) : isSlideshowActive ? (
        // Slideshow Mode
        <div
          className="fixed inset-0 bg-black flex items-center justify-center"
          onMouseMove={() => {
            setControlsVisible(true);
            if (controlsTimeoutRef.current)
              clearTimeout(controlsTimeoutRef.current);
            controlsTimeoutRef.current = setTimeout(
              () => setControlsVisible(false),
              3000
            );
          }}
        >
          {images.map((image, index) => (
            <img
              key={image.id}
              src={`http://localhost:8080/uploads/${encodeURIComponent(
                album.name
              )}/${encodeURIComponent(image.file_name)}`}
              alt={`Slide ${index + 1}`}
              className={`w-full h-full object-contain absolute transition-opacity duration-1000 ease-in-out ${
                index === currentSlide ? "opacity-100" : "opacity-0"
              }`}
            />
          ))}
          {controlsVisible && (
            <div className="absolute bottom-4 left-1/2 transform -translate-x-1/2 bg-gray-800 bg-opacity-50 p-4 rounded-lg flex items-center space-x-4">
              <Button
                variant="outline"
                onClick={() => setCurrentSlide((prev) => Math.max(prev - 1, 0))}
                disabled={currentSlide === 0}
              >
                <ArrowLeft />
              </Button>
              <Button
                variant="outline"
                onClick={() =>
                  setCurrentSlide((prev) =>
                    prev + 1 < images.length ? prev + 1 : prev
                  )
                }
                disabled={currentSlide === images.length - 1}
              >
                <ArrowRight />
              </Button>
              <Button
                variant="outline"
                onClick={() => setIsPaused((prev) => !prev)}
              >
                {isPaused ? <Play /> : <Pause />}
              </Button>
              <Button variant="outline" onClick={toggleFullscreen}>
                {document.fullscreenElement ? <Minimize2 /> : <Maximize2 />}
              </Button>
              <Button
                variant="outline"
                onClick={() => {
                  setIsSlideshowActive(false);
                  if (document.fullscreenElement) {
                    document.exitFullscreen();
                  }
                }}
              >
                <X />
              </Button>
              <div className="flex items-center space-x-2">
                <label className="text-sm text-white">Transition Time:</label>
                <input
                  type="number"
                  value={transitionTime / 1000}
                  onChange={(e) =>
                    setTransitionTime(
                      Math.max(1000, Number(e.target.value) * 1000)
                    )
                  }
                  className="w-12 bg-gray-700 text-white text-center rounded"
                  min="1"
                />
                <span className="text-sm text-white">s</span>
              </div>
            </div>
          )}
        </div>
      ) : (
        // Normal mode
        <div>
          <div className="flex justify-between items-center mb-8">
            <Button
              variant="outline"
              onClick={() => router.push("/")}
              className="text-sm"
            >
              ← Back to Home
            </Button>
            <div className="text-center">
              <h1 className="text-3xl font-bold">{album.name}</h1>
              <p className="text-sm text-gray-500 mt-1 italic">{album.date}</p>
            </div>
            <Button
              variant="outline"
              onClick={() => {
                setIsSlideshowActive(true);
                toggleFullscreen();
              }}
              className="text-sm"
            >
              Slideshow
            </Button>
          </div>

          <p className="text-md text-gray-500 text-center mb-8 leading-relaxed">
            {album.description}
          </p>

          <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-6">
            {images.map((image, index) => (
              <Card key={image.id} onClick={() => handleImageClick(index)}>
                <div className="relative aspect-[4/3] cursor-pointer">
                  <img
                    src={`http://localhost:8080/uploads/${encodeURIComponent(
                      album.name
                    )}/${encodeURIComponent(image.file_name)}`}
                    alt={`Image ${image.id}`}
                    className="absolute inset-0 w-full h-full object-cover rounded-lg"
                    loading="lazy"
                  />
                </div>
              </Card>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

export default AlbumPage;
