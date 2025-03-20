import Link from "next/link";
import Image from 'next/image';

export default function Page() {
  const days = ['S', 'M', 'T', 'W', 'T', 'F', 'S'];
  const currentDate = new Date();
  const currentWeekNumber = Math.ceil((currentDate.getDate() - currentDate.getDay()) / 7);
  const monthNames = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
  const currentMonth = monthNames[currentDate.getMonth()];

  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 p-6 overflow-y-auto h-auto">
      {/* Calendar card */}
      <Link href="/calendars">
        <div className="card shadow-sm border p-4 h-full flex flex-col">
          <div className="card-body flex-grow">
            <div className="flex justify-between items-center">
              <h2 className="card-title">Calendar - Week {currentWeekNumber + 1}, {currentMonth}</h2>
              <span className="material-symbols-outlined text-lg">add</span>
            </div>
            <div className="flex flex-wrap gap-2 mt-4">
              {days.map((day, index) => (
                <div
                  key={index}
                  className={`flex border rounded h-10 items-center px-2 py-1 text-sm ${index === currentDate.getDay() ? 'bg-primary text-white' : ''}`}
                >
                  {day} - {index - currentDate.getDay() + currentDate.getDate()}
                </div>
              ))}
            </div>
            <p className="mt-4 text-sm sm:text-base">No event to display this week.</p>
          </div>
        </div>
      </Link>

      {/* Messages card */}
      <Link href="/messages">
        <div className="card shadow-sm border p-4 h-full flex flex-col">
          <div className="card-body flex-grow">
            <div className="flex justify-between items-center">
              <h2 className="card-title">Messages</h2>
              <span className="material-symbols-outlined text-lg">mail</span>
            </div>
            <div className="flex items-center mt-4 border rounded-lg p-4 overflow-hidden">
              <Image
                alt="Profile"
                src="https://randomuser.me/api/portraits/women/1.jpg"
                width={40}
                height={40}
                className="rounded-full"
              />
              <div className="ml-4 overflow-hidden">
                <h2 className="font-bold">Private message</h2>
                <p className="text-sm truncate">Hey Evan! Tu viens toujours à la soirée ce soir? 🎉</p>
              </div>
            </div>
          </div>
        </div>
      </Link>

      {/* E-learning card */}
      <Link href="/e-learning">
        <div className="card shadow-sm border p-4 h-full flex flex-col">
          <div className="card-body flex-grow">
            <div className="flex justify-between items-center">
              <h2 className="card-title">E-learning</h2>
              <span className="material-symbols-outlined text-lg">school</span>
            </div>
            <div className="mt-4 border rounded-lg p-4">
              <h2 className="text-lg">User training</h2>
              <div className="w-full bg-gray-300 rounded-full h-4 mt-3 relative">
                <div className="bg-primary h-4 rounded-full" style={{ width: '45%' }}></div>
                <p className="absolute inset-0 flex items-center justify-center text-sm text-black">45%</p>
              </div>
            </div>
          </div>
        </div>
      </Link>

      {/* Emails card */}
      <Link href="/emails">
        <div className="card shadow-sm border p-4 h-full flex flex-col">
          <div className="card-body flex-grow">
            <div className="flex justify-between items-center">
              <h2 className="card-title">Emails</h2>
              <span className="material-symbols-outlined text-lg">inbox</span>
            </div>
            <div className="flex items-center justify-between mt-4 border rounded-lg p-4">
              <h3 className="font-bold text-sm sm:text-base">4 New mails</h3>
              <span className="material-symbols-outlined text-lg">mail</span>
            </div>
          </div>
        </div>
      </Link>

      {/* Files card */}
      <Link href="/files">
        <div className="card shadow-sm border p-4 h-full flex flex-col">
          <div className="card-body flex-grow">
            <div className="flex justify-between items-center">
              <h2 className="card-title">Files</h2>
              <span className="material-symbols-outlined text-lg">upload_file</span>
            </div>
            <div className="flex items-center justify-between mt-4 border rounded-lg p-4">
              <div className="flex gap-2 items-center">
                <span className="material-symbols-outlined text-lg">description</span>
                <h3 className="font-bold text-sm sm:text-base truncate">Internship_report.pdf</h3>
              </div>
              <span className="material-symbols-outlined text-lg">download</span>
            </div>
          </div>
        </div>
      </Link>

      {/* Projects card */}
      <Link href="/projects">
        <div className="card shadow-sm border p-4 h-full flex flex-col">
          <div className="card-body flex-grow">
            <div className="flex justify-between items-center">
              <h2 className="card-title">Projects</h2>
              <span className="material-symbols-outlined text-lg">extension</span>
            </div>
            <div className="flex items-center justify-between mt-4 border rounded-lg p-4">
              <h3 className="text-sm sm:text-base">No projects available.</h3>
            </div>
          </div>
        </div>
      </Link>
    </div>
  );
}