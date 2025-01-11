"use client";

import Image from "next/image";
import { Card } from "@/components/ui/card";
import { useState, useEffect } from "react";
import Link from "next/link";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { ArrowUpDown } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";

// Define TypeScript interfaces for Album and AlbumImage
interface Album {
  id: string;
  name: string;
  description: string;
  date?: string;
  number_of_images: number;
  thumbnail?: string;
  camera_model?: string;
  lens_model?: string;
  aperture?: string;
}

interface AlbumImage {
  id: string;
  album_id: string;
  file_name: string;
}

const HomePage = () => {
  const [albums, setAlbums] = useState<Album[]>([]);
  const [filteredAlbums, setFilteredAlbums] = useState<Album[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [sortKey, setSortKey] = useState<"date" | "name">("date"); // Default sort by date
  const [searchQuery, setSearchQuery] = useState<string>("");

  useEffect(() => {
    const fetchAlbums = async () => {
      try {
        const response = await fetch("http://localhost:8080/api/albums");
        if (!response.ok) {
          throw new Error("Failed to fetch albums");
        }

        const albumsData: Album[] = await response.json();

        // Fetch mode metadata for each album and attach it
        const albumsWithMetadata = await Promise.all(
          albumsData.map(async (album) => {
            const imagesResponse = await fetch(
              `http://localhost:8080/api/albums/${album.id}`
            );
            const modeMetadataResponse = await fetch(
              `http://localhost:8080/api/albums/${album.id}/mode-metadata`
            );

            let modeMetadata = {
              camera_model: undefined,
              lens_model: undefined,
              aperture: undefined,
            };

            if (modeMetadataResponse.ok) {
              modeMetadata = await modeMetadataResponse.json();
              console.log("modeMetadata", modeMetadata);
            }

            const imagesData: { images: AlbumImage[] } = imagesResponse.ok
              ? await imagesResponse.json()
              : { images: [] };

            const firstImage = imagesData.images[0]?.file_name;
            const thumbnail = firstImage
              ? `http://localhost:8080/uploads/${encodeURIComponent(
                  album.name
                )}/${encodeURIComponent(firstImage)}`
              : "https://via.placeholder.com/300x200";

            return { ...album, thumbnail, ...modeMetadata };
          })
        );

        // Sort albums by date (newest first) initially
        albumsWithMetadata.sort(
          (a, b) =>
            new Date(b.date || "").getTime() - new Date(a.date || "").getTime()
        );

        setAlbums(albumsWithMetadata);
        setFilteredAlbums(albumsWithMetadata); // Initialize filtered albums
      } catch (err: unknown) {
        if (err instanceof Error) {
          console.error("Error fetching albums:", err.message);
          setError(err.message);
        } else {
          console.error("Unexpected error fetching albums");
          setError("Unexpected error");
        }
      } finally {
        setLoading(false);
      }
    };

    fetchAlbums();
  }, []);

  // Handle sorting
  const handleSort = (key: "date" | "name") => {
    const sortedAlbums = [...filteredAlbums].sort((a, b) => {
      if (key === "date") {
        return (
          new Date(b.date || "").getTime() - new Date(a.date || "").getTime()
        ); // Newest first
      }
      return a.name.localeCompare(b.name);
    });
    setFilteredAlbums(sortedAlbums);
    setSortKey(key);
  };

  // Handle search filtering
  const handleSearch = (e: React.ChangeEvent<HTMLInputElement>) => {
    const query = e.target.value.toLowerCase();
    setSearchQuery(query);
    const filtered = albums.filter((album) =>
      album.name.toLowerCase().includes(query)
    );
    setFilteredAlbums(filtered);
  };

  if (loading) {
    return <p className="text-center">Loading albums...</p>;
  }

  if (error) {
    return <p className="text-center text-red-500">Error: {error}</p>;
  }

  return (
    <div className="container mx-auto p-6">
      <div className="flex justify-between items-center mb-8">
        <h1 className="text-3xl font-bold">Photo Albums</h1>
        <div className="flex space-x-4">
          <Link href="/admin" className="text-sm text-gray-500 hover:underline">
            Admin Panel
          </Link>
        </div>
      </div>
      {/* Search and Sorting Controls */}
      <div className="flex justify-between items-center mb-4">
        {/* Search Bar */}
        <Input
          type="text"
          placeholder="Search albums..."
          value={searchQuery}
          onChange={handleSearch}
          className="w-full max-w-xs"
        />

        {/* Sorting Dropdown */}
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="outline" className="w-[180px] justify-between">
              {sortKey === "date" ? "Sort by Date" : "Sort by Name"}
              <ArrowUpDown className="ml-2 h-4 w-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent className="w-[180px]">
            <DropdownMenuItem onClick={() => handleSort("date")}>
              Sort by Date
            </DropdownMenuItem>
            <DropdownMenuItem onClick={() => handleSort("name")}>
              Sort by Name
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
      <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-6">
        {filteredAlbums.map((album) => (
          <Card
            key={album.id}
            className="relative overflow-hidden rounded-lg shadow-lg hover:shadow-xl transition-shadow"
          >
            <Link href={`/album/${album.id}`} className="block">
              {/* Background Image with Aspect Ratio */}
              <div className="relative aspect-[4/3]">
                <Image
                  src={album.thumbnail || "n/a"}
                  alt={`Thumbnail for ${album.name}`}
                  fill
                  style={{ objectFit: "cover" }}
                  loading="lazy"
                  sizes="(max-width: 768px) 100vw, (max-width: 1200px) 50vw, 33vw"
                />
                {/* Badge displaying number of images */}
                <Badge
                  variant="outline"
                  className="absolute top-2 right-2 bg-black/50 text-white"
                >
                  {album.number_of_images}{" "}
                  {album.number_of_images === 1 ? "image" : "images"}
                </Badge>
                {/* Stacked badges for camera model, lens model, and aperture */}
                <div className="absolute top-2 right-2 flex flex-col items-end space-y-1">
                  <Badge variant="outline" className="bg-black/50 text-white">
                    {album.number_of_images}{" "}
                    {album.number_of_images === 1 ? "image" : "images"}
                  </Badge>
                  {album.camera_model && (
                    <Badge variant="outline" className="bg-black/50 text-white">
                      {album.camera_model}
                    </Badge>
                  )}
                  {album.lens_model && (
                    <Badge variant="outline" className="bg-black/50 text-white">
                      {album.lens_model}
                    </Badge>
                  )}
                  {album.aperture && (
                    <Badge variant="outline" className="bg-black/50 text-white">
                      {album.aperture}
                    </Badge>
                  )}
                </div>
              </div>

              {/* Content with Overlay */}
              <div className="absolute bottom-0 left-0 right-0 p-4 bg-gradient-to-t from-black/80 to-transparent backdrop-blur-sm border-gray-700">
                <div className="flex justify-between items-center">
                  <h2 className="text-lg font-semibold text-white">
                    {album.name}
                  </h2>
                  <p className="text-sm text-gray-300">{album.date}</p>
                </div>
                <p className="text-sm text-gray-300 line-clamp-2 mt-2">
                  {album.description}
                </p>
              </div>
            </Link>
          </Card>
        ))}
      </div>
    </div>
  );
};

export default HomePage;
