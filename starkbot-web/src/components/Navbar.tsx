import { Github, Monitor } from 'lucide-react'

export function Navbar() {
  return (
    <nav className="fixed top-0 left-0 right-0 z-50 backdrop-blur-md bg-slate-950/80 border-b border-slate-800/50">
      <div className="max-w-6xl mx-auto px-6 py-4 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 bg-gradient-to-br from-stark-400 to-stark-600 rounded-xl flex items-center justify-center shadow-lg">
            <Monitor className="w-6 h-6 text-white" />
          </div>
          <span className="text-xl font-bold">StarkBot</span>
        </div>
        <a
          href="https://github.com/ethereumdegen/stark-bot"
          target="_blank"
          rel="noopener noreferrer"
          className="flex items-center gap-2 px-4 py-2 bg-slate-800 hover:bg-slate-700 rounded-lg transition-all duration-300 border border-slate-700 hover:border-stark-500"
        >
          <Github className="w-5 h-5" />
          <span className="hidden sm:inline">GitHub</span>
        </a>
      </div>
    </nav>
  )
}
