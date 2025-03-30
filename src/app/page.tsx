'use client';

import { useEffect, useState } from "react";
import Link from "next/link";
import { useModule } from "@/context/ModuleContext";

interface Module {
  id: number;
  name: string;
  description: string;
  version: string;
  url: string;
}

export default function Page() {
  const [modules, setModules] = useState<Module[]>([]);
  const [error, setError] = useState<string | null>(null);
  const { currentModule } = useModule(); // 👈 Écoute du module sélectionné

  useEffect(() => {
    fetch("/api/modules")
      .then((response) => response.json())
      .then((data) => {
        if (data.status === "success") {
          setModules(data.data);
        } else {
          setError(data.message);
        }
      })
      .catch(() => setError("Failed to fetch modules"));
  }, []);

  if (error) return <p>Error: {error}</p>;
  if (modules.length === 0) return <p>Loading...</p>;

  if (currentModule) {
    return (
      <iframe
        id="module-iframe"
        src={currentModule.url}
        className="w-full h-full"
        style={{ border: "none" }}
        title={currentModule.name}
      />
    );
  } else {
    return (
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 p-6 overflow-y-auto h-auto">
        {modules.map((module) => (
          <Link key={module.id} href={`/${module.name.toLowerCase()}`}>
            <div className="card shadow-sm border p-4 h-full flex flex-col">
              <div className="card-body flex-grow">
                <h2 className="card-title">{module.name}</h2>
                <p className="mt-4 text-sm sm:text-base">{module.description}</p>
              </div>
            </div>
          </Link>
        ))}
      </div>
    );
  }
}