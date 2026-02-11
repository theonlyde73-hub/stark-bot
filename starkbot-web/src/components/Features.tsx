import { AlertTriangle } from 'lucide-react'
import ChatDemo from './ChatDemo'
import MindMapDemo from './MindMapDemo'

export function Features() {
  return (
    <section id="features" className="py-20 px-6">
      <div className="max-w-6xl mx-auto">
        {/* Chat Demo Section */}
        <div className="mb-16">
          <h2 className="text-3xl sm:text-4xl font-bold text-center mb-4 text-white">
            See It In Action
          </h2>
          <p className="text-white/50 text-center mb-8 max-w-2xl mx-auto">
            This is what StarkBot looks like when you run your own instance
          </p>
          <div className="chat-demo-glow rounded-xl">
            <ChatDemo />
          </div>
        </div>

        {/* Mind Map Demo Section */}
        <div className="mb-16">
          <h2 className="text-3xl sm:text-4xl font-bold text-center mb-4 text-white">
            Autonomous Mind Map
          </h2>
          <p className="text-white/50 text-center mb-8 max-w-2xl mx-auto">
            Define random actions for your bot to execute on heartbeat pulses. Build a tree of possible behaviors.
          </p>
          <div className="chat-demo-glow rounded-xl">
            <MindMapDemo />
          </div>
        </div>

        {/* Warning Banner */}
        <div className="mb-12 p-6 bg-white/5 border border-white/20 rounded-xl">
          <div className="flex items-start gap-4">
            <div className="flex-shrink-0">
              <AlertTriangle className="w-8 h-8 text-white/70" />
            </div>
            <div>
              <h3 className="text-xl font-bold text-white/90 mb-2">WARNING</h3>
              <p className="text-white/60 leading-relaxed">
                Starkbot is in active development and not production-ready software.
                Starkbot is not responsible for data loss or security intrusions.
                Always run Starkbot in a sandboxed VPS container.
                Feel free to contribute to development with a{' '}
                <a
                  href="https://github.com/ethereumdegen/stark-bot"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-white hover:text-white/80 underline"
                >
                  pull request
                </a>.
              </p>
            </div>
          </div>
        </div>

      </div>
    </section>
  )
}
