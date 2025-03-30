'use client';

import { usePathname } from "next/navigation";
import Image from "next/image";
import Link from "next/link";
import { useEffect, useState } from "react";
import { useModule } from "@/context/ModuleContext";

interface Module {
  id: number;
  name: string;
  description: string;
  version: string;
  url: string;
}

export default function Navbar() {
  const pathname = usePathname();
  const isActive = (path: string) => (pathname === path ? "bg-primary text-white" : "");

  const [links, setLinks] = useState<Module[]>([]);
  const { setCurrentModule } = useModule();

  useEffect(() => {
    fetch("/api/modules")
      .then((res) => res.json())
      .then((data) => {
        if (data.status === "success") {
          setLinks(data.data);
        }
      })
      .catch((error) => console.error("Error fetching modules:", error));
  }, []);

  return (
    <div className="navbar h-16 bg-base-100 shadow-sm pl-6">
      <div className="navbar-start">
        <Image src="/icon.svg" alt="Mairie360" width={50} height={50} className="rounded" />
        <Link className="ml-4 text-xl font-bold hidden lg:block" href="/">Mairie360</Link>
      </div>

      <div className="navbar-center hidden lg:flex">
        <ul className="menu menu-horizontal px-1">
          {links.map((link) => (
            <li key={link.id}>
              <button
                className={`btn btn-ghost ${isActive(link.url)}`}
                onClick={() => setCurrentModule(link)}
              >
                {link.name}
              </button>
            </li>
          ))}
        </ul>
      </div>

      <div className="navbar-end">

        <button className="btn btn-ghost btn-circle">
          <svg xmlns="http://www.w3.org/2000/svg" className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"> <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" /> </svg>
        </button>

        <button className="btn btn-ghost btn-circle">
          <div className="indicator">
            <svg xmlns="http://www.w3.org/2000/svg" className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"> <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9" /> </svg>
            <span className="badge badge-xs badge-primary indicator-item"></span>
          </div>
        </button>

        <div className="flex gap-2">
          <div className="dropdown dropdown-end">
            <div tabIndex={0} role="button" className="btn btn-ghost btn-circle avatar">
              <div className="w-10 rounded-full">
                <img
                  alt="Tailwind CSS Navbar component"
                  src="https://img.daisyui.com/images/stock/photo-1534528741775-53994a69daeb.webp" />
              </div>
            </div>
            <ul
              tabIndex={0}
              className="menu menu-sm dropdown-content bg-base-100 rounded-box z-1 mt-3 w-52 p-2 shadow">
              <li>
                <a className="justify-between">
                  Profile
                  <span className="badge">New</span>
                </a>
              </li>
              <li><a>Settings</a></li>
              <li><a>Logout</a></li>
            </ul>
          </div>
        </div>
      </div>
    </div>
  );
}