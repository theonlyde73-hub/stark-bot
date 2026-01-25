import { Cloud } from 'lucide-react'

const steps = [
  {
    number: 1,
    title: 'Clone the Repo',
    description: 'Get the source code from GitHub',
  },
  {
    number: 2,
    title: 'Configure',
    description: 'Set up your environment variables',
  },
  {
    number: 3,
    title: 'Run with Docker',
    description: 'Build and run with a single command',
  },
]

export function GetStarted() {
  return (
    <section className="py-20 px-6 bg-slate-900/30">
      <div className="max-w-5xl mx-auto text-center">
        <h2 className="text-3xl sm:text-4xl font-bold mb-4">
          <span className="gradient-text">Get Started in Minutes</span>
        </h2>
        <p className="text-slate-400 mb-16 max-w-2xl mx-auto">
          Setting up StarkBot is quick and easy
        </p>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-8 mb-16">
          {steps.map((step) => (
            <div key={step.number}>
              <div className="w-12 h-12 bg-stark-500 rounded-full flex items-center justify-center text-xl font-bold mx-auto mb-4">
                {step.number}
              </div>
              <h3 className="text-xl font-bold mb-2">{step.title}</h3>
              <p className="text-slate-400">{step.description}</p>
            </div>
          ))}
        </div>

        {/* Docker Code Block */}
        <div className="bg-slate-950 rounded-xl border border-slate-800 p-6 text-left max-w-2xl mx-auto">
          <div className="flex items-center gap-2 mb-4">
            <div className="w-3 h-3 bg-red-500 rounded-full" />
            <div className="w-3 h-3 bg-yellow-500 rounded-full" />
            <div className="w-3 h-3 bg-green-500 rounded-full" />
            <span className="ml-2 text-slate-500 text-sm">terminal</span>
          </div>
          <code className="text-sm text-slate-300 font-mono block space-y-1">
            <div><span className="text-stark-400">$</span> git clone https://github.com/ethereumdegen/stark-bot</div>
            <div><span className="text-stark-400">$</span> cd stark-bot</div>
            <div><span className="text-stark-400">$</span> cp .env.example .env</div>
            <div><span className="text-stark-400">$</span> docker compose up -d</div>
          </code>
        </div>

        {/* DigitalOcean Section */}
        <div className="mt-20">
          <h3 className="text-2xl sm:text-3xl font-bold mb-4">
            <span className="gradient-text">Deploy to the Cloud</span>
          </h3>
          <p className="text-slate-400 mb-8 max-w-2xl mx-auto">
            Deploy StarkBot directly to DigitalOcean App Platform for a fully managed, serverless experience
          </p>

          <div className="bg-slate-900/50 backdrop-blur-sm rounded-2xl border border-slate-800 p-8 max-w-2xl mx-auto">
            <div className="flex items-center justify-center gap-3 mb-6">
              <Cloud className="w-8 h-8 text-stark-400" />
              <span className="text-xl font-bold">DigitalOcean App Platform</span>
            </div>

            <ol className="text-left text-slate-400 space-y-4 mb-8">
              <li className="flex gap-3">
                <span className="text-stark-400 font-bold">1.</span>
                <span>Fork the <a href="https://github.com/ethereumdegen/stark-bot" target="_blank" rel="noopener noreferrer" className="text-stark-400 hover:text-stark-300 underline">stark-bot repository</a> to your GitHub account</span>
              </li>
              <li className="flex gap-3">
                <span className="text-stark-400 font-bold">2.</span>
                <span>Go to <a href="https://cloud.digitalocean.com/apps" target="_blank" rel="noopener noreferrer" className="text-stark-400 hover:text-stark-300 underline">DigitalOcean App Platform</a> and click "Create App"</span>
              </li>
              <li className="flex gap-3">
                <span className="text-stark-400 font-bold">3.</span>
                <span>Connect your GitHub and select the forked repository</span>
              </li>
              <li className="flex gap-3">
                <span className="text-stark-400 font-bold">4.</span>
                <span>Configure your environment variables (API keys, etc.)</span>
              </li>
              <li className="flex gap-3">
                <span className="text-stark-400 font-bold">5.</span>
                <span>Deploy! DigitalOcean auto-detects the Dockerfile</span>
              </li>
            </ol>

            <a
              href="https://cloud.digitalocean.com/apps/new?repo=https://github.com/ethereumdegen/stark-bot/tree/main"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-2 px-6 py-3 bg-[#0080FF] hover:bg-[#0069d9] text-white font-semibold rounded-lg transition-all duration-300"
            >
              <svg className="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
                <path d="M12.04 0C5.408-.02.005 5.37.005 11.992h4.638c0-4.923 4.882-8.731 10.064-6.9a6.81 6.81 0 014.16 4.16c1.83 5.182-1.977 10.064-6.9 10.064v-3.601l-4.927 4.926 4.927 4.928v-3.607c6.618-.007 11.993-5.418 11.967-12.042C23.907 5.376 18.562.02 12.04 0z"/>
              </svg>
              Deploy to DigitalOcean
            </a>
          </div>
        </div>
      </div>
    </section>
  )
}
