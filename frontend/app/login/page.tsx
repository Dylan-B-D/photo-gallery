'use client'

import { useState } from "react"
import { useRouter } from "next/navigation"
import { setToken } from "@/utils/auth"
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { LockIcon } from 'lucide-react'
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Label } from "@/components/ui/label"
import { Input } from "@/components/ui/input"

export default function LoginPage() {
  const [username, setUsername] = useState("")
  const [password, setPassword] = useState("")
  const [error, setError] = useState("")
  const router = useRouter()

  interface LoginResponse {
    token: string
    error?: string
  }

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault()
    setError("")

    try {
      const response = await fetch("http://localhost:8080/api/login", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ username, password }),
      })

      const data: LoginResponse = await response.json()

      if (response.ok) {
        console.log("LoginPage: Setting token", data.token)
        setToken(data.token)
        console.log("LoginPage: Redirecting to admin")
        router.push("/admin")
      } else {
        setError(data.error || "Login failed")
      }
    } catch (err) {
      console.error("LoginPage: Error during login", err)
      setError("An error occurred. Please try again.")
    }
  }

  return (
    <div className="min-h-screen flex flex-col items-center justify-center p-4">
      <Card className="w-full max-w-md border dark:border-gray-700">
        <CardHeader className="space-y-1">
          <CardTitle className="text-2xl font-bold text-center">Admin Login</CardTitle>
          <CardDescription className="text-center dark:text-gray-400">
            Enter your credentials to access the admin panel
          </CardDescription>
        </CardHeader>
        <form onSubmit={handleSubmit}>
          <CardContent className="space-y-4">
            {error && (
              <Alert variant="destructive">
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}
            <div className="space-y-2">
              <Label htmlFor="username" className="dark:text-gray-200">Username</Label>
              <Input
                id="username"
                type="text"
                placeholder="Enter your username"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                required
                className="dark:bg-gray-700 dark:text-white dark:placeholder-gray-400"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="password" className="dark:text-gray-200">Password</Label>
              <Input
                id="password"
                type="password"
                placeholder="Enter your password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                required
                className="dark:bg-gray-700 dark:text-white dark:placeholder-gray-400"
              />
            </div>
          </CardContent>
          <CardFooter>
            <Button className="w-full" type="submit">
              <LockIcon className="mr-2 h-4 w-4" /> Sign In
            </Button>
          </CardFooter>
        </form>
      </Card>
      <Button variant="outline" onClick={() => router.push("/")} className="w-full max-w-md mt-4 text-sm">
        ← Back to Home
      </Button>
    </div>
  )
}