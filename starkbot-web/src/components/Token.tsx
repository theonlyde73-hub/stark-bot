import { Coins, ExternalLink, Copy, Check } from 'lucide-react'
import { useState } from 'react'

const CONTRACT_ADDRESS = '0x587Cd533F418825521f3A1daa7CCd1E7339A1B07'

export function Token() {
  const [copied, setCopied] = useState(false)

  const copyAddress = () => {
    navigator.clipboard.writeText(CONTRACT_ADDRESS)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <section className="py-16 px-6">
      <div className="max-w-4xl mx-auto">
        <div className="bg-gradient-to-br from-blue-500/10 to-purple-500/10 rounded-2xl border border-blue-500/20 p-8">
          <div className="flex flex-col md:flex-row items-center gap-8">
            {/* Token Icon */}
            <div className="flex-shrink-0">
              <div className="w-24 h-24 bg-gradient-to-br from-blue-500 to-blue-700 rounded-2xl flex items-center justify-center shadow-lg shadow-blue-500/30">
                <Coins className="w-12 h-12 text-white" />
              </div>
            </div>

            {/* Token Info */}
            <div className="flex-1 text-center md:text-left">
              <div className="flex items-center justify-center md:justify-start gap-3 mb-2">
                <h3 className="text-2xl font-bold text-white">$STARKBOT</h3>
                <span className="px-2 py-0.5 bg-blue-500/20 text-blue-400 text-xs font-medium rounded-full">
                  BASE
                </span>
              </div>
              <p className="text-white/60 mb-4">
                The official StarkBot token on Base. Community-driven, powering the future of Web3 AI agents.
              </p>

              {/* Contract Address */}
              <div className="flex items-center justify-center md:justify-start gap-2 mb-6">
                <code className="text-sm text-white/50 font-mono bg-white/5 px-3 py-1.5 rounded-lg">
                  {CONTRACT_ADDRESS.slice(0, 6)}...{CONTRACT_ADDRESS.slice(-4)}
                </code>
                <button
                  onClick={copyAddress}
                  className="p-1.5 text-white/50 hover:text-white bg-white/5 hover:bg-white/10 rounded-lg transition-colors"
                  title="Copy address"
                >
                  {copied ? <Check className="w-4 h-4 text-green-400" /> : <Copy className="w-4 h-4" />}
                </button>
              </div>

              {/* Action Buttons */}
              <div className="flex flex-wrap items-center justify-center md:justify-start gap-3">
                <a
                  href="https://app.uniswap.org/swap?chain=base&outputCurrency=0x587Cd533F418825521f3A1daa7CCd1E7339A1B07"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="inline-flex items-center gap-2 px-5 py-2.5 bg-gradient-to-r from-blue-500 to-blue-600 hover:from-blue-400 hover:to-blue-500 text-white font-semibold rounded-xl transition-all duration-300 transform hover:scale-105 shadow-lg hover:shadow-blue-500/25"
                >
                  Buy on Uniswap
                  <ExternalLink className="w-4 h-4" />
                </a>
                <a
                  href="https://www.geckoterminal.com/base/pools/0x0d64a8e0d28626511cc23fc75b81c2f03e222b14f9b944b60eecc3f4ddabeddc"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="inline-flex items-center gap-2 px-5 py-2.5 bg-white/5 hover:bg-white/10 text-white font-medium rounded-xl transition-all duration-300 border border-white/10 hover:border-white/20"
                >
                  Chart
                  <ExternalLink className="w-4 h-4" />
                </a>
                <a
                  href="https://clanker.world/clanker/0x587Cd533F418825521f3A1daa7CCd1E7339A1B07"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="inline-flex items-center gap-2 px-5 py-2.5 bg-white/5 hover:bg-white/10 text-white font-medium rounded-xl transition-all duration-300 border border-white/10 hover:border-white/20"
                >
                  Clanker
                  <ExternalLink className="w-4 h-4" />
                </a>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  )
}
