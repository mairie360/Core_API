import type { Metadata } from "next";
import "./globals.css";
import Navbar from "@/components/navbar";

export const metadata: Metadata = {
  title: "Core",
  description: "The Core's module.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className="h-full">
      <head>
        <meta name="apple-mobile-web-app-title" content="Mairie360" />
        <link
          rel="stylesheet"
          href="https://fonts.googleapis.com/css2?family=Material+Symbols+Outlined&display=swap"
        />
      </head>
      <body className="h-full">
        {/* Navbar */}
        <header className="w-full h-16">
          <Navbar />
        </header>

        {/* Main */}
        <main className="h-[calc(100vh-7vh)]">
          {children}
        </main>
      </body>
    </html>
  );
}

