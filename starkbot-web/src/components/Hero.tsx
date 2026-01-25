import { Github, ChevronDown, Monitor } from 'lucide-react'

export function Hero() {
  return (
    <section className="pt-32 pb-20 px-6">
      <div className="max-w-4xl mx-auto text-center">
        {/* Mascot/Logo */}
        <div className="mb-8 animate-float">
          <div className="w-32 h-32 mx-auto bg-gradient-to-br from-stark-400 via-stark-500 to-stark-600 rounded-3xl flex items-center justify-center glow transform rotate-3 hover:rotate-0 transition-transform duration-500">
            <div className="relative">
              <Monitor className="w-16 h-16 text-white" strokeWidth={1.5} />
              <div className="absolute top-6 left-4 flex gap-4">
                <div className="w-2 h-2 bg-white rounded-full" />
                <div className="w-2 h-2 bg-white rounded-full" />
              </div>
            </div>
          </div>
        </div>

        {/* Title */}
        <h1 className="text-5xl sm:text-7xl font-black mb-6 tracking-tight">
          <span className="gradient-text">StarkBot</span>
        </h1>

        {/* Tagline */}
        <p className="text-stark-400 text-xl sm:text-2xl font-semibold uppercase tracking-widest mb-2">
          The Cloud AI That Actually Gets Things Done.
        </p>
       

        {/* Description */}
        <p className="text-slate-400 text-lg sm:text-xl max-w-2xl mx-auto leading-relaxed mb-12">
          Automate your workflows, manage tasks, and boost productivity with an intelligent AI assistant.
          Open source, self-hostable, and ready to integrate with your favorite tools.
        </p>

        {/* CTA Buttons */}
        <div className="flex flex-col sm:flex-row gap-4 justify-center">
          <a
            href="https://github.com/ethereumdegen/stark-bot"
            target="_blank"
            rel="noopener noreferrer"
            className="px-8 py-4 bg-gradient-to-r from-stark-500 to-stark-600 hover:from-stark-400 hover:to-stark-500 text-white font-semibold rounded-xl transition-all duration-300 transform hover:scale-105 shadow-lg hover:shadow-stark-500/25 flex items-center justify-center gap-3"
          >
            <Github className="w-6 h-6" />
            View on GitHub
          </a>
          <a
            href="#features"
            className="px-8 py-4 bg-slate-800 hover:bg-slate-700 text-white font-semibold rounded-xl transition-all duration-300 border border-slate-700 hover:border-stark-500 flex items-center justify-center gap-2"
          >
            Learn More
            <ChevronDown className="w-5 h-5" />
          </a>
        </div>
      </div>
    </section>
  )
}
