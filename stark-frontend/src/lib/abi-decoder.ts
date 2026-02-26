/**
 * ABI Decoder utility - matches function selectors against known ABIs
 * and decodes calldata to show human-readable parameters
 */
import { Interface, FunctionFragment, Result } from 'ethers';

// Import ABIs
import erc20Abi from '@abis/erc20.json';
import wethAbi from '@abis/weth.json';
import settlerAbi from '@abis/0x_settler.json';
import safeAbi from '@abis/safe.json';
import aavePoolAbi from '@abis/aave_pool.json';
import commonWrappersAbi from '@abis/common_wrappers.json';

interface AbiFile {
  name: string;
  description?: string;
  abi: Array<{
    name: string;
    type: string;
    inputs?: Array<{ name: string; type: string }>;
    outputs?: Array<{ name: string; type: string }>;
    stateMutability?: string;
  }>;
}

interface DecodedParam {
  name: string;
  type: string;
  value: string;
}

export interface DecodedFunction {
  abiName: string;
  functionName: string;
  signature: string;
  params: DecodedParam[];
  /** If this is a wrapper (exec, execTransaction), the decoded inner call from the bytes param */
  innerCall?: DecodedFunction | null;
  /** For wrapper functions: the actual target address from the decoded params */
  innerTo?: string;
  /** For wrapper functions: the actual value from the decoded params */
  innerValue?: string;
}

// Map of selector -> { abiName, interface, fragment }
interface SelectorEntry {
  abiName: string;
  iface: Interface;
  fragment: FunctionFragment;
}

const selectorMap = new Map<string, SelectorEntry>();

// Build the selector map from all ABIs
function buildSelectorMap() {
  const abiFiles: AbiFile[] = [erc20Abi, wethAbi, settlerAbi, safeAbi, aavePoolAbi, commonWrappersAbi];

  for (const abiFile of abiFiles) {
    try {
      const iface = new Interface(abiFile.abi);

      // Iterate over all functions in the ABI
      for (const fragment of iface.fragments) {
        if (fragment.type === 'function') {
          const funcFragment = fragment as FunctionFragment;
          const selector = iface.getFunction(funcFragment.name)?.selector;
          if (selector) {
            // Only add if not already present (first ABI wins for duplicates)
            if (!selectorMap.has(selector)) {
              selectorMap.set(selector, {
                abiName: abiFile.name,
                iface,
                fragment: funcFragment
              });
            }
          }
        }
      }
    } catch (e) {
      console.warn(`Failed to parse ABI for ${abiFile.name}:`, e);
    }
  }
}

// Initialize on module load
buildSelectorMap();

/**
 * Format a decoded value for display
 */
function formatValue(value: unknown, type: string): string {
  if (value === null || value === undefined) {
    return 'null';
  }

  // Handle BigInt/bigint values
  if (typeof value === 'bigint') {
    // For uint256 amounts, try to format as decimal with potential token formatting
    if (type === 'uint256' || type === 'uint128' || type === 'uint64') {
      const str = value.toString();
      // If it's a large number (likely wei), show both raw and formatted
      if (str.length > 15) {
        const eth = Number(value) / 1e18;
        if (eth >= 0.000001) {
          return `${str} (${eth.toFixed(6)} in 18 decimals)`;
        }
      }
      return str;
    }
    return value.toString();
  }

  // Handle addresses
  if (type === 'address' && typeof value === 'string') {
    return value;
  }

  // Handle bytes
  if (type === 'bytes' || type.startsWith('bytes')) {
    const bytesStr = String(value);
    if (bytesStr.length > 66) {
      return `${bytesStr.slice(0, 66)}... (${(bytesStr.length - 2) / 2} bytes)`;
    }
    return bytesStr;
  }

  // Handle arrays
  if (Array.isArray(value)) {
    return JSON.stringify(value.map(v => formatValue(v, 'unknown')));
  }

  return String(value);
}

/**
 * Known wrapper function patterns that contain inner calls in a bytes param.
 * Maps function name -> array of possible param name patterns to try.
 * The first address param is usually `to`/`dest`/`target`, the bytes param is the inner calldata.
 */
const WRAPPER_FUNCTIONS: Record<string, { toParams: string[]; valueParams: string[]; dataParams: string[] }> = {
  // Safe execTransaction(to, value, data, operation, ...)
  'execTransaction': { toParams: ['to'], valueParams: ['value'], dataParams: ['data'] },
  // Generic exec/execute - module executors, ERC-7579, etc.
  'exec': { toParams: ['to', 'dest', 'target'], valueParams: ['value'], dataParams: ['data', 'callData'] },
  'execute': { toParams: ['to', 'dest', 'target'], valueParams: ['value'], dataParams: ['data', 'callData'] },
};

function findParamIndex(inputs: readonly { name: string; type: string }[], names: string[], type: string): number {
  for (const name of names) {
    const idx = inputs.findIndex(inp => inp.name === name && inp.type === type);
    if (idx >= 0) return idx;
  }
  // Fallback: find first param of the target type
  return inputs.findIndex(inp => inp.type === type);
}

/**
 * Check if a hex string looks like it could be calldata (has a 4-byte selector)
 */
function looksLikeCalldata(hexStr: string): boolean {
  const cleaned = hexStr.startsWith('0x') ? hexStr.slice(2) : hexStr;
  // Must have at least 4 bytes (8 hex chars) for a selector
  return cleaned.length >= 8 && /^[0-9a-fA-F]+$/.test(cleaned);
}

/**
 * Try to decode calldata using our known ABIs.
 * For wrapper functions (exec, execTransaction), also attempts to decode the inner call.
 * Returns null if no matching ABI is found.
 */
export function decodeCalldata(data: string): DecodedFunction | null {
  if (!data || data === '0x' || data.length < 10) {
    return null;
  }

  // Extract selector (first 4 bytes)
  const selector = data.slice(0, 10).toLowerCase();
  const entry = selectorMap.get(selector);

  if (!entry) {
    return null;
  }

  try {
    // Decode the calldata
    const decoded: Result = entry.iface.decodeFunctionData(entry.fragment.name, data);

    // Build params array with names and formatted values
    const params: DecodedParam[] = entry.fragment.inputs.map((input, i) => ({
      name: input.name || `param${i}`,
      type: input.type,
      value: formatValue(decoded[i], input.type)
    }));

    // Build the signature string
    const inputTypes = entry.fragment.inputs.map(i => i.type).join(',');
    const signature = `${entry.fragment.name}(${inputTypes})`;

    const result: DecodedFunction = {
      abiName: entry.abiName,
      functionName: entry.fragment.name,
      signature,
      params
    };

    // Check if this is a wrapper function and try to decode inner call
    const wrapperConfig = WRAPPER_FUNCTIONS[entry.fragment.name];
    if (wrapperConfig) {
      const dataParamIndex = findParamIndex(entry.fragment.inputs, wrapperConfig.dataParams, 'bytes');
      const toParamIndex = findParamIndex(entry.fragment.inputs, wrapperConfig.toParams, 'address');
      const valueParamIndex = findParamIndex(entry.fragment.inputs, wrapperConfig.valueParams, 'uint256');

      if (dataParamIndex >= 0) {
        const innerData = String(decoded[dataParamIndex]);
        if (innerData && innerData !== '0x' && looksLikeCalldata(innerData)) {
          result.innerCall = decodeCalldata(innerData);
        }
      }

      if (toParamIndex >= 0) {
        result.innerTo = String(decoded[toParamIndex]);
      }
      if (valueParamIndex >= 0) {
        result.innerValue = formatValue(decoded[valueParamIndex], 'uint256');
      }
    } else {
      // Even for non-configured wrappers, check any bytes params that look like calldata
      entry.fragment.inputs.forEach((input, i) => {
        if (input.type === 'bytes' && !result.innerCall) {
          const bytesVal = String(decoded[i]);
          if (bytesVal && bytesVal !== '0x' && looksLikeCalldata(bytesVal) && bytesVal.length >= 10) {
            const inner = decodeCalldata(bytesVal);
            if (inner) {
              result.innerCall = inner;
            }
          }
        }
      });
    }

    return result;
  } catch (e) {
    console.warn('Failed to decode calldata:', e);
    return null;
  }
}

/**
 * Get the function selector from calldata
 */
export function getSelector(data: string): string | null {
  if (!data || data === '0x' || data.length < 10) {
    return null;
  }
  return data.slice(0, 10).toLowerCase();
}

/**
 * Check if we have an ABI for a given selector
 */
export function hasAbiForSelector(selector: string): boolean {
  return selectorMap.has(selector.toLowerCase());
}
