import { NextApiRequest } from 'next';

export const setToken = (token: string) => {
    document.cookie = `authToken=${token}; path=/; max-age=${60 * 60 * 24}; SameSite=Lax`;
};

export const getToken = (req?: NextApiRequest) => {
    console.log("getToken: Getting token");

    if (typeof window !== 'undefined') {
        // Client-side
        console.log("getToken: Running on client side");
        const cookies = document.cookie.split('; ').reduce((acc, cookie) => {
            const [key, value] = cookie.split('=');
            acc[key] = value;
            return acc;
        }, {} as { [key: string]: string });
        const token = cookies.authToken;
        return token;
    } else if (req) {
        // Server-side
        console.log("getToken: Running on server side");
        
        // Get the token directly from request.cookies
        const token = req?.cookies?.authToken;
        return token;
    }

    console.log("getToken: Returning null");
    return null;
};

export const removeToken = () => {
    console.log("Removing token");
    document.cookie = 'authToken=; path=/; max-age=0';
};

export const isAuthenticated = async (req?: NextApiRequest) => {
    console.log("isAuthenticated: Checking if user is authenticated");
    
    const token = getToken(req);
    if (!token) {
        console.log("isAuthenticated: No token found");
        return false;
    }

    const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL;

    try {
        const response = await fetch(`${API_BASE_URL}/api/verify`, {
            headers: {
                'Authorization': `Bearer ${token}`
            }
        });
        if (response.ok) {
            console.log("isAuthenticated: Token is valid");
            return true;
        } else {
            console.log("isAuthenticated: Token is invalid");
            return false;
        }
    } catch {
        return false;
    }
};
