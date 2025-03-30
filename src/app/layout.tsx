import type { Metadata } from "next";
import "./globals.css";

import { ModuleProvider } from "@/context/ModuleContext";

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
      </head>
      <ModuleProvider>
        <body className="h-full">
          <header className="w-full h-16">
            <Navbar />
          </header>
          <main className="h-[calc(100vh-7vh)]">
            {children}
          </main>
        </body>
      </ModuleProvider>
    </html>
  );
}

