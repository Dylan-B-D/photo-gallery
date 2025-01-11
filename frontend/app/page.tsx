"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
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
import { Input } from "@/components/ui/input"; // Assuming you have an Input component
import { Badge } from "@/components/ui/badge";

// Define TypeScript interfaces for Album and AlbumImage
interface Album {
  id: string;
  name: string;
  description: string;
  date?: string;
  number_of_images: number;
  thumbnail?: string; // Optional field for thumbnail URL
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
        console.log("Fetched albums:", albumsData); // Log fetched albums

        const albumsWithThumbnails = await Promise.all(
          albumsData.map(async (album) => {
            const imagesResponse = await fetch(
              `http://localhost:8080/api/albums/${album.id}`
            );
            if (imagesResponse.ok) {
              const imagesData: { images: AlbumImage[] } =
                await imagesResponse.json();
              console.log(
                `Fetched images for album ${album.name}:`,
                imagesData
              );

              const firstImage = imagesData.images[0]?.file_name;
              const thumbnail = firstImage
                ? `http://localhost:8080/uploads/${encodeURIComponent(
                    album.name
                  )}/${encodeURIComponent(firstImage)}`
                : "https://via.placeholder.com/300x200";

              console.log(`Thumbnail URL for album ${album.name}:`, thumbnail);
              return { ...album, thumbnail };
            }

            console.warn(`No images found for album ${album.name}`);
            return {
              ...album,
              thumbnail: "https://via.placeholder.com/300x200",
            };
          })
        );

        // Sort albums by date (newest first) initially
        albumsWithThumbnails.sort(
          (a, b) =>
            new Date(b.date || "").getTime() - new Date(a.date || "").getTime()
        );

        setAlbums(albumsWithThumbnails);
        setFilteredAlbums(albumsWithThumbnails); // Initialize filtered albums
      } catch (err: any) {
        console.error("Error fetching albums:", err.message);
        setError(err.message);
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
        <img
          src={album.thumbnail}
          alt={`Thumbnail for ${album.name}`}
          className="absolute inset-0 w-full h-full object-cover"
          loading="lazy"
        />
        {/* Badge displaying number of images */}
        <Badge
          variant="outline"
          className="absolute top-2 right-2 bg-black/50 text-white"
        >
          {album.number_of_images}{" "}
          {album.number_of_images === 1 ? "image" : "images"}
        </Badge>
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
