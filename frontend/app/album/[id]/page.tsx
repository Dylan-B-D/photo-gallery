"use client";

import Image from "next/image";
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
  Loader2,
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

interface ImageMetadata {
  image_id: string;
  camera_make?: string;
  camera_model?: string;
  lens_model?: string;
  iso?: number;
  aperture?: number;
  shutter_speed?: string;
  focal_length?: number;
  light_source?: string;
  date_created?: string;
  file_size?: number;
}

const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL;
const UPLOAD_BASE_URL = process.env.NEXT_PUBLIC_UPLOAD_URL;

const AlbumPage = () => {
  const { id } = useParams();
  const router = useRouter();

  const [album, setAlbum] = useState<Album | null>(null);
  const [images, setImages] = useState<AlbumImage[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [isSlideshowActive, setIsSlideshowActive] = useState(false);
  const [currentSlide, setCurrentSlide] = useState(0);
  const [isPaused, setIsPaused] = useState(false);
  const [controlsVisible, setControlsVisible] = useState(true);
  const [transitionTime, setTransitionTime] = useState(3000); // Default transition time in milliseconds
  const [isViewingMode, setIsViewingMode] = useState(false); // Viewing mode flag
  const [showInfo, setShowInfo] = useState(false); // Info panel toggle

  const [imageMetadata, setImageMetadata] = useState<ImageMetadata | null>(
    null
  ); // Image metadata state
  const [isMetadataLoading, setIsMetadataLoading] = useState(false); // Metadata loading state
  const [metadataError, setMetadataError] = useState<string | null>(null); // Metadata error state

  const controlsTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  useEffect(() => {
    if (!id) return;

    const fetchAlbumAndImages = async () => {
      try {
        const response = await fetch(`${API_BASE_URL}/api/albums/${id}`);
        if (!response.ok) throw new Error("Failed to fetch album images");

        const data: { album: Album; images: AlbumImage[] } =
          await response.json();

        setAlbum(data.album);
        setImages(data.images);
      } catch (err) {
        if (err instanceof Error) {
          setError(err.message);
        } else {
          setError("An unknown error occurred");
        }
      }
    };

    fetchAlbumAndImages();
  }, [id]);

  // Slideshow logic
  useEffect(() => {
    let slideshowInterval: NodeJS.Timeout | null = null;

    if (isSlideshowActive && !isPaused) {
      slideshowInterval = setInterval(() => {
        setCurrentSlide(
          (prev) => (prev + 1 < images.length ? prev + 1 : 0) // Loop back to the first image
        );
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

  const fetchImageMetadata = async (imageId: string) => {
    setIsMetadataLoading(true);
    setMetadataError(null);

    try {
      const response = await fetch(
        `${API_BASE_URL}/api/images/${imageId}/metadata`
      );

      if (!response.ok) {
        throw new Error("Failed to fetch image metadata");
      }

      const data: { metadata: ImageMetadata } = await response.json();
      setImageMetadata(data.metadata);
    } catch (err) {
      if (err instanceof Error) {
        setMetadataError(err.message);
      } else {
        setMetadataError("An unknown error occurred");
      }
      setImageMetadata(null);
    } finally {
      setIsMetadataLoading(false);
    }
  };

  useEffect(() => {
    if (showInfo && images[currentSlide]) {
      fetchImageMetadata(images[currentSlide].id);
    }
  }, [showInfo, currentSlide, images]);

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
          <div className="absolute top-4 left-4 flex items-center space-x-4 z-20">
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
              className="absolute left-4 z-20 text-white"
            >
              <ArrowLeft size={48} />
            </Button>
          )}

          {/* Image */}
          <div className="relative w-full h-full">
            <Image
              src={`${UPLOAD_BASE_URL}/uploads/${encodeURIComponent(
                album.name
              )}/${encodeURIComponent(images[currentSlide].file_name)}`}
              alt={`Image ${currentSlide + 1}`}
              fill
              style={{ objectFit: "contain" }}
              className="z-10"
            />
          </div>

          {/* Next Button */}
          {currentSlide < images.length - 1 && (
            <Button
              variant="ghost"
              onClick={() =>
                setCurrentSlide((prev) =>
                  prev + 1 < images.length ? prev + 1 : prev
                )
              }
              className="absolute right-4 z-20 text-white"
            >
              <ArrowRight size={48} />
            </Button>
          )}

          {/* Download and Info Buttons */}
          <div className="absolute bottom-4 right-4 flex items-center space-x-4 z-20">
            <Button
              variant="outline"
              onClick={async () => {
                const url = `${UPLOAD_BASE_URL}/uploads/${encodeURIComponent(
                  album.name
                )}/${encodeURIComponent(images[currentSlide].file_name)}`;

                try {
                  const response = await fetch(url);
                  if (!response.ok) {
                    throw new Error(
                      `Failed to fetch image. Status: ${response.status}`
                    );
                  }

                  const blob = await response.blob();
                  const tempUrl = URL.createObjectURL(blob);

                  const link = document.createElement("a");
                  link.href = tempUrl;
                  link.download = images[currentSlide].file_name;
                  link.click();

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
            <div className="absolute bottom-16 right-4 bg-gray-800 bg-opacity-90 p-6 rounded-lg text-white w-80 shadow-lg">
              {isMetadataLoading ? (
                <div className="flex items-center justify-center">
                  <Loader2 className="animate-spin" />
                </div>
              ) : metadataError ? (
                <p className="text-red-500 text-center">
                  Error: {metadataError}
                </p>
              ) : imageMetadata ? (
                <div>
                  <h2 className="text-xl font-semibold mb-4 border-b border-gray-600 pb-2">
                    Image Metadata
                  </h2>

                  {/* Camera Info Section */}
                  <div className="mb-4">
                    <div className="space-y-1 text-sm">
                      <p>
                        <strong>Make:</strong>{" "}
                        {imageMetadata.camera_make || "N/A"}
                      </p>
                      <p>
                        <strong>Model:</strong>{" "}
                        {imageMetadata.camera_model || "N/A"}
                      </p>
                      <p>
                        <strong>Lens:</strong>{" "}
                        {imageMetadata.lens_model || "N/A"}
                      </p>
                    </div>
                  </div>

                  {/* Technical Details Section */}
                  <div className="mb-4">
                    <div className="space-y-1 text-sm">
                      <p>
                        <strong>ISO:</strong> {imageMetadata.iso || "N/A"}
                      </p>
                      <p>
                        <strong>Aperture:</strong>{" "}
                        {imageMetadata.aperture
                          ? `f/${imageMetadata.aperture}`
                          : "N/A"}
                      </p>
                      <p>
                        <strong>Shutter Speed:</strong>{" "}
                        {imageMetadata.shutter_speed || "N/A"}
                      </p>
                      <p>
                        <strong>Focal Length:</strong>{" "}
                        {imageMetadata.focal_length
                          ? `${imageMetadata.focal_length}mm`
                          : "N/A"}
                      </p>
                      <p>
                        <strong>Light Source:</strong>{" "}
                        {imageMetadata.light_source || "N/A"}
                      </p>
                    </div>
                  </div>

                  {/* Image Details Section */}
                  <div>
                    <div className="space-y-1 text-sm">
                      <p>
                        <strong>Date Created:</strong>{" "}
                        {imageMetadata.date_created || "N/A"}
                      </p>
                      <p>
                        <strong>File Size:</strong>{" "}
                        {imageMetadata.file_size
                          ? `${(imageMetadata.file_size / 1024).toFixed(2)} KB`
                          : "N/A"}
                      </p>
                    </div>
                  </div>
                </div>
              ) : (
                <p className="text-center">No metadata available.</p>
              )}
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
            <div
              key={image.id}
              className={`w-full h-full object-contain absolute transition-opacity duration-1000 ease-in-out ${
                index === currentSlide ? "opacity-100" : "opacity-0"
              }`}
            >
              <Image
                src={`${UPLOAD_BASE_URL}/uploads/${encodeURIComponent(
                  album.name
                )}/${encodeURIComponent(image.file_name)}`}
                alt={`Slide ${index + 1}`}
                fill
                style={{ objectFit: "contain" }}
              />
            </div>
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
              <h1 className="font-serif italic text-3xl font-bold">{album.name}</h1>
              <p className="text-sm text-gray-500 mt-1">{album.date}</p>
            </div>
            <Button
              variant="outline"
              onClick={() => {
                setCurrentSlide(0); 
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
                  <Image
                    src={`${UPLOAD_BASE_URL}/uploads/${encodeURIComponent(
                      album.name
                    )}/${encodeURIComponent(image.file_name)}`}
                    alt={`Image ${image.id}`}
                    fill
                    style={{ objectFit: "cover" }}
                    className="absolute inset-0 w-full h-full object-cover rounded-lg"
                    loading="lazy"
                    sizes="(max-width: 768px) 100vw, (max-width: 1200px) 50vw, 33vw"
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
