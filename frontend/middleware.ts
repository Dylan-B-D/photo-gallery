import { isAuthenticated } from '@/utils/auth';
import { NextRequest, NextResponse } from 'next/server';

export async function middleware(request: NextRequest): Promise<NextResponse> {
    console.log("Middleware: Protecting admin routes");

    // Protect admin routes
    if (request.nextUrl.pathname.startsWith('/admin')) {
        const isAuth: boolean = await isAuthenticated(request);
        console.log("Middleware: isAuthenticated result:", isAuth);

        if (!isAuth) {
            console.log("Middleware: Redirecting to login");
            return NextResponse.redirect(new URL('/login', request.url));
        }
    }

    console.log("Middleware: Proceeding to next middleware or route handler");
    return NextResponse.next();
}

export const config = {
    matcher: '/admin/:path*',
};
