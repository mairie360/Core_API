'use client';

import { createContext, useContext, useState, ReactNode } from "react";

interface Module {
  id: number;
  name: string;
  description: string;
  version: string;
  url: string;
}

interface ModuleContextType {
  currentModule: Module | null;
  setCurrentModule: (module: Module) => void;
}

const ModuleContext = createContext<ModuleContextType | undefined>(undefined);

export function ModuleProvider({ children }: { children: ReactNode }) {
  const [currentModule, setCurrentModule] = useState<Module | null>(null);

  return (
    <ModuleContext.Provider value={{ currentModule, setCurrentModule }}>
      {children}
    </ModuleContext.Provider>
  );
}

export function useModule() {
  const context = useContext(ModuleContext);
  if (!context) throw new Error("useModule must be used within a ModuleProvider");
  return context;
}