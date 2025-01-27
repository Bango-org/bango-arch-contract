import React, { useState, useEffect, useCallback } from 'react';
import { RpcConnection, MessageUtil, PubkeyUtil, Instruction, Message, UtxoMetaSchema } from '@saturnbtcio/arch-sdk';
import { Copy, Check, AlertCircle } from 'lucide-react';
import { Buffer } from 'buffer';
import { useWallet } from '../hooks/useWallet';
import * as borsh from 'borsh';
import { v4 as uuidv4 } from "uuid"

window.Buffer = Buffer;

// Environment variables for configuration
const client = new RpcConnection((import.meta as any).env.VITE_RPC_URL || 'http://localhost:9002');
const PROGRAM_PUBKEY = (import.meta as any).env.VITE_PROGRAM_PUBKEY;
const WALL_ACCOUNT_PUBKEY = (import.meta as any).env.VITE_WALL_ACCOUNT_PUBKEY;

class GraffitiMessage {
  constructor(
    public timestamp: number,
    public name: string,
    public message: string
  ) { }

  static schema = new Map([
    [
      GraffitiMessage,
      {
        kind: 'struct',
        fields: [
          ['timestamp', 'i64'],
          ['name', ['u8', 16]],
          ['message', ['u8', 64]]
        ]
      }
    ]
  ]);
}

// Define the schema for the wall containing messages
class GraffitiWall {
  constructor(public messages: GraffitiMessage[]) { }

  static schema = new Map([
    [
      GraffitiWall,
      {
        kind: 'struct',
        fields: [
          ['messages', [GraffitiMessage]]
        ]
      }
    ]
  ]);
}



const GraffitiWallComponent: React.FC = () => {
  // State management
  const wallet = useWallet();
  const [error, setError] = useState<string | null>(null);
  const [isAccountCreated, setIsAccountCreated] = useState(false);
  const [isProgramDeployed, setIsProgramDeployed] = useState(false);
  const [wallData, setWallData] = useState<GraffitiMessage[]>([]);

  // Form state
  const [message, setMessage] = useState('');
  const [name, setName] = useState('');
  const [isFormValid, setIsFormValid] = useState(false);
  const [copied, setCopied] = useState(false);

  // Convert account pubkey once
  const accountPubkey = PubkeyUtil.fromHex(WALL_ACCOUNT_PUBKEY);

  // Utility Functions
  const copyToClipboard = () => {
    navigator.clipboard.writeText(`arch-cli account create --name <unique_name> --program-id ${PROGRAM_PUBKEY}`);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  // Check if the program is deployed on the network
  const checkProgramDeployed = useCallback(async () => {
    try {
      const pubkeyBytes = PubkeyUtil.fromHex(PROGRAM_PUBKEY);
      const accountInfo = await client.readAccountInfo(pubkeyBytes);
      if (accountInfo) {
        setIsProgramDeployed(true);
        setError(null);
      }
    } catch (error) {
      console.error('Error checking program:', error);
      setError('The Arch Graffiti program has not been deployed to the network yet. Please run `arch-cli deploy`.');
    }
  }, []);

  // Check if the wall account exists
  const checkAccountCreated = useCallback(async () => {
    try {
      const pubkeyBytes = PubkeyUtil.fromHex(WALL_ACCOUNT_PUBKEY);
      const accountInfo = await client.readAccountInfo(pubkeyBytes);
      if (accountInfo) {
        setIsAccountCreated(true);
        setError(null);
      }
    } catch (error) {
      console.error('Error checking account:', error);
      setIsAccountCreated(false);
      setError('The wall account has not been created yet. Please run the account creation command.');
    }
  }, []);


  // Fetch and parse wall messages
  const fetchWallData = useCallback(async () => {
    try {
      const userAccount = await client.readAccountInfo(accountPubkey);
      if (!userAccount) {
        setError('Account not found.');
        return;
      }
      const wallData = userAccount.data;

      console.log(`Wall data: ${wallData}`);

      // If data is empty or invalid length, just set empty messages without error
      if (!wallData || wallData.length < 4) {
        setWallData([]);
        setError(null); // Clear any existing errors
        return;
      }

      const schema = {
        struct: {
          owner: { array: { type: 'u8', len: 32 } }, // 32-byte array
          status: 'u8', // Enum as u8 (MintStatus)
          supply: 'u64', // u64 as BigInt
          circulating_supply: 'u64', // u64 as BigInt
          ticker: 'string', // Token ticker (String)
          decimals: 'u8', // Unsigned 8-bit integer
          token_metadata: {
            map: {
              key: 'string', // Key is a string
              value: { array: { type: 'u8', len: 32 } }, // Value is a 32-byte array
            },
          },
          balances: {
            map: {
              key: { array: { type: 'u8', len: 32 } }, // Key is a 32-byte array (Pubkey)
              value: 'u64', // Value is u64 (BigInt)
            },
          },
        },
      };
      

      const data = borsh.deserialize(schema, wallData);

      console.log(data);


    } catch (error) {
      console.error('Error fetching wall data:', error);
      setError(`Failed to fetch wall data: ${error instanceof Error ? error.message : String(error)}`);
    }
  }, []);

  // Initialize component
  useEffect(() => {
    checkProgramDeployed();
    checkAccountCreated();
  }, [checkAccountCreated, checkProgramDeployed]);

  // Set up polling for wall data
  useEffect(() => {
    if (!isAccountCreated) return;

    fetchWallData();
    
    const interval = setInterval(fetchWallData, 5000);
    return () => clearInterval(interval);
  }, [isAccountCreated, fetchWallData]);

  // Message handlers
  const handleNameChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newName = e.target.value;
    const bytes = new TextEncoder().encode(newName);

    if (bytes.length <= 16) {
      setName(newName);
      setIsFormValid(newName.trim() !== '' && message.trim() !== '');
    }
  };

  const handleMessageChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newMessage = e.target.value;
    const bytes = new TextEncoder().encode(newMessage);

    if (bytes.length <= 64) {
      setMessage(newMessage);
      setIsFormValid(name.trim() !== '' && newMessage.trim() !== '');
    }
  };



  const handleCreateEvent = async () => {

    try {
      console.log("arr2:")

      const uniqueId = new Uint8Array(32).fill(0); // Fill with your ID bytes
      const uniqueIdBytes = new TextEncoder().encode(uuidv4().replaceAll("-", ""));
      uniqueId.set(uniqueIdBytes.slice(0, 32));


      const expiryTimestamp = 1689422272; // 24 hours from now
      const numOutcomes = 3;

      const schema = {
        struct: {
          function_number: 'u8',
          unique_id: { array: { type: 'u8', len: 32 } },
          expiry_timestamp: 'u32',
          num_outcomes: 'u8',
        }
      };


      let dataStruct = {
        function_number: 1,
        unique_id: Array.from(uniqueId),
        expiry_timestamp: expiryTimestamp,
        num_outcomes: numOutcomes,
      };

      const data = borsh.serialize(schema, dataStruct);

      console.log("arr2:", data)

      const instruction: Instruction = {
        program_id: PubkeyUtil.fromHex(PROGRAM_PUBKEY),
        accounts: [
          {
            pubkey: accountPubkey,
            is_signer: false,
            is_writable: true
          },
          {
            pubkey: PubkeyUtil.fromHex(wallet.publicKey!),
            is_signer: true,
            is_writable: false
          }
        ],
        data: data,
      };

      const messageObj: Message = {
        signers: [PubkeyUtil.fromHex(wallet.publicKey!)],
        instructions: [instruction],
      };

      const messageHash = MessageUtil.hash(messageObj);
      const signature = await wallet.signMessage(Buffer.from(messageHash).toString('hex'));
      // let signature = await window.unisat.signMessage(Buffer.from(messageHash).toString('hex'));
      const signatureBytes = new Uint8Array(Buffer.from(signature, 'base64')).slice(2);


      const result = await client.sendTransaction({
        version: 0,
        signatures: [signatureBytes],
        message: messageObj,
      });

    } catch (error) {
      console.error('Error creating event:', error);
      setError(`Failed to create event: ${error instanceof Error ? error.message : String(error)}`);
    }
  };


  const handleCreateNewToken = async () => {
    try {

      const owner = new Uint8Array(32).fill(0); // Fill with your ID bytes
      const ownerBytes = new TextEncoder().encode(wallet.publicKey!);
      owner.set(ownerBytes.slice(0, 32));

      const supply = BigInt(1000000);
      const ticker = "Bango1T"
      const decimals = 10;


      const schema = {
        struct: {
          function_number: 'u8',
          owner: { array: { type: 'u8', len: 32 } },
          supply: 'u64',
          ticker: 'string',
          decimals: 'u8'
        }
      };

      const data2 = {
        function_number: 5,
        owner: Array.from(owner),
        supply,
        ticker,
        decimals: decimals
      };

      const serialized_data = borsh.serialize(schema, data2);

      const instruction: Instruction = {
        program_id: PubkeyUtil.fromHex(PROGRAM_PUBKEY),
        accounts: [
          {
            pubkey: accountPubkey,
            is_signer: false,
            is_writable: true
          }
        ],
        data: serialized_data,
      };

      const messageObj: Message = {
        signers: [PubkeyUtil.fromHex(wallet.publicKey!)],
        instructions: [instruction],
      };

      const messageHash = MessageUtil.hash(messageObj);
      const signature = await wallet.signMessage(Buffer.from(messageHash).toString('hex'));
      // let signature = await window.unisat.signMessage(Buffer.from(messageHash).toString('hex'));
      const signatureBytes = new Uint8Array(Buffer.from(signature, 'base64')).slice(2);

      const result = await client.sendTransaction({
        version: 0,
        signatures: [signatureBytes],
        message: messageObj,
      });

      console.log(result, "====")
    }

    catch (error) {
      console.error('Error creating Token:', error);
      setError(`Failed to create Token : ${error instanceof Error ? error.message : String(error)}`);
    }


  }


  const handleCreateBalanceAccount = async () => {

    try {
      
      // const sendCoin = await Wallet.request()
      console.log(wallet.address)
      // const contractAddress = await client.getAccountAddress(PubkeyUtil.fromHex(PROGRAM_PUBKEY))

      const uniqueId = new Uint8Array(32).fill(0); // Fill with your ID bytes
      const uniqueIdBytes = new TextEncoder().encode(uuidv4().replaceAll("-", ""));
      uniqueId.set(uniqueIdBytes.slice(0, 32));

      const schema = {
        struct: {
          function_number: 'u8',
          uid: { array: { type: 'u8', len: 32 } },
          amount: 'u64',
        }
      };

      const data2 = {
        function_number: 6,
        uid: uniqueId,
        amount: BigInt(10000),
      };

      const serialized_data = borsh.serialize(schema, data2);

      const instruction: Instruction = {
        program_id: PubkeyUtil.fromHex(PROGRAM_PUBKEY),
        accounts: [
          {
            pubkey: accountPubkey,
            is_signer: false,
            is_writable: true,
          },
          {
            pubkey: PubkeyUtil.fromHex(wallet.publicKey!),
            is_signer: true,
            is_writable: false
          }
        ],
        data: serialized_data,
      };

      const messageObj: Message = {
        signers: [PubkeyUtil.fromHex(wallet.publicKey!)],
        instructions: [instruction],
      };

      const messageHash = MessageUtil.hash(messageObj);
      const signature = await wallet.signMessage(Buffer.from(messageHash).toString('hex'));
      // let signature = await window.unisat.signMessage(Buffer.from(messageHash).toString('hex'));
      const signatureBytes = new Uint8Array(Buffer.from(signature, 'base64')).slice(2);


      const result = await client.sendTransaction({
        version: 0,
        signatures: [signatureBytes],
        message: messageObj,
      });

    } catch (error) {
      console.error('Error creating event:', error);
      setError(`Failed to create event: ${error instanceof Error ? error.message : String(error)}`);
    }
  };




  const handleAddToWall = async () => {
    if (!message.trim() || !name.trim() || !isAccountCreated || !wallet.isConnected) {
      setError("Name and message are required, account must be created, and wallet must be connected.");
      return;
    }

    try {
      const data = serializeGraffitiData(name, message);

      const instruction: Instruction = {
        program_id: PubkeyUtil.fromHex(PROGRAM_PUBKEY),
        accounts: [
          {
            pubkey: PubkeyUtil.fromHex(wallet.publicKey!),
            is_signer: true,
            is_writable: false
          },
          {
            pubkey: accountPubkey,
            is_signer: false,
            is_writable: true
          },
        ],
        data: new Uint8Array(data),
      };

      const messageObj: Message = {
        signers: [PubkeyUtil.fromHex(wallet.publicKey!)],
        instructions: [instruction],
      };

      console.log(`Pubkey: ${PubkeyUtil.fromHex(wallet.publicKey!)}`);
      const messageBytes = MessageUtil.serialize(messageObj);
      console.log(`Message hash: ${MessageUtil.hash(messageObj).toString()}`);
      const signature = await wallet.signMessage(Buffer.from(MessageUtil.hash(messageObj)).toString('hex'));
      console.log(`Signature: ${signature}`);

      // Take last 64 bytes of base64 decoded signature
      const signatureBytes = new Uint8Array(Buffer.from(signature, 'base64')).slice(2);
      console.log(`Signature bytes: ${signatureBytes}`);

      const result = await client.sendTransaction({
        version: 0,
        signatures: [signatureBytes],
        message: messageObj,
      });

      if (result) {
        await fetchWallData();
        setMessage('');
        setName('');
      }
    } catch (error) {
      console.error('Error adding to wall:', error);
      setError(`Failed to add to wall: ${error instanceof Error ? error.message : String(error)}`);
    }
  };

  const serializeGraffitiData = (name: string, message: string): number[] => {
    // Create fixed-size arrays
    const nameArray = new Uint8Array(16).fill(0);
    const messageArray = new Uint8Array(64).fill(0);

    // Convert strings to bytes
    const nameBytes = new TextEncoder().encode(name);
    const messageBytes = new TextEncoder().encode(message);

    // Copy bytes into fixed-size arrays (will truncate if too long)
    nameArray.set(nameBytes.slice(0, 16));
    messageArray.set(messageBytes.slice(0, 64));

    // Create the params object matching the Rust struct
    const params = {
      name: Array.from(nameArray),
      message: Array.from(messageArray)
    };

    // Define the schema for borsh serialization
    const schema = {
      struct: {
        name: { array: { type: 'u8', len: 16 } },
        message: { array: { type: 'u8', len: 64 } }
      }
    };

    return Array.from(borsh.serialize(schema, params));
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      if (isFormValid) {
        handleAddToWall();
      }
    }
  };


  return (
    <div className="bg-gradient-to-br from-arch-gray to-gray-900 p-8 rounded-lg shadow-lg max-w-4xl mx-auto">
      <h2 className="text-3xl font-bold mb-6 text-center text-arch-white">Graffiti Wall</h2>


      {!wallet.isConnected ? (
        <button
          onClick={wallet.connect}
          className="w-full mb-4 bg-arch-orange text-arch-black font-bold py-2 px-4 rounded-lg hover:bg-arch-white transition duration-300"
        >
          Connect Wallet
        </button>
      ) : (
        <button
          onClick={wallet.disconnect}
          className="w-full mb-4 bg-gray-600 text-arch-white font-bold py-2 px-4 rounded-lg hover:bg-gray-700 transition duration-300"
        >
          Disconnect Wallet
        </button>
      )}


      {!isAccountCreated ? (
        <div className="bg-arch-black p-6 rounded-lg">
          <h3 className="text-2xl font-bold mb-4 text-arch-white">Account Setup Required</h3>
          <p className="text-arch-white mb-4">To participate in the Graffiti Wall, please create an account using the Arch CLI:</p>
          <div className="relative mb-4">
            <pre className="bg-gray-800 p-4 rounded-lg text-arch-white overflow-x-auto">
              <code>
                arch-cli account create --name &lt;unique_name&gt; --program-id {PROGRAM_PUBKEY}
              </code>
            </pre>
            <button
              onClick={copyToClipboard}
              className="absolute top-2 right-2 p-2 bg-arch-orange text-arch-black rounded hover:bg-arch-white transition-colors duration-300"
              title="Copy to clipboard"
            >
              {copied ? <Check size={20} /> : <Copy size={20} />}
            </button>
          </div>
          <p className="text-arch-white mb-4">Run this command in your terminal to set up your account.</p>

        </div>
      ) : (
        <div className="flex flex-col md:flex-row gap-8">
          <div className="flex-1">
            <div className="bg-arch-black p-6 rounded-lg">
              <h3 className="text-2xl font-bold mb-4 text-arch-white">Add to Wall</h3>
              <input
                type="text"
                value={name}
                onChange={handleNameChange}
                placeholder="Your Name (required, max 16 bytes)"
                className="w-full px-3 py-2 bg-arch-gray text-arch-white rounded-md focus:outline-none focus:ring-2 focus:ring-arch-orange mb-2"
                required
              />
              <textarea
                value={message}
                onChange={handleMessageChange}
                onKeyDown={handleKeyDown}
                placeholder="Your Message (required, max 64 bytes)"
                className="w-full px-3 py-2 bg-arch-gray text-arch-white rounded-md focus:outline-none focus:ring-2 focus:ring-arch-orange mb-2"
                required
              />
              <button
                onClick={handleCreateEvent}
                className={`w-full font-bold py-2 px-4 rounded-lg transition duration-300 ${isFormValid
                  ? 'bg-arch-orange text-arch-black hover:bg-arch-white hover:text-arch-orange'
                  : 'bg-gray-500 text-gray-300 cursor-not-allowed'
                  }`}
              >
                Create event
              </button>
              <button
                onClick={handleCreateNewToken}
                className={`w-full font-bold py-2 px-4 rounded-lg transition duration-300 ${isFormValid
                  ? 'bg-arch-orange text-arch-black hover:bg-arch-white hover:text-arch-orange'
                  : 'bg-gray-500 text-gray-300 cursor-not-allowed'
                  }`}
              >
                Create Token
              </button>

              <button
                onClick={handleCreateBalanceAccount}
                className={`w-full font-bold py-2 px-4 rounded-lg transition duration-300 ${isFormValid
                  ? 'bg-arch-orange text-arch-black hover:bg-arch-white hover:text-arch-orange'
                  : 'bg-gray-500 text-gray-300 cursor-not-allowed'
                  }`}
              >
                Mint Token
              </button>
            </div>
          </div>

          <div className="flex-1">
            <div className="bg-arch-black p-6 rounded-lg">
              <h3 className="text-2xl font-bold mb-4 text-arch-white">Wall Messages</h3>
              <div className="space-y-4 max-h-96 overflow-y-auto">
                {wallData.map((item, index) => (
                  <div key={index} className="bg-arch-gray p-3 rounded-lg">
                    <p className="font-bold text-arch-orange">{new Date(item.timestamp * 1000).toLocaleString()}</p>
                    <p className="text-arch-white"><span className="font-semibold">{item.name}:</span> {item.message}</p>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      )}

      {error && (
        <div className="mt-6 p-4 bg-red-500 text-white rounded-lg">
          <div className="flex items-center mb-2">
            <AlertCircle className="w-6 h-6 mr-2" />
            <p className="font-bold">Program Error</p>
          </div>
          <p>{error}</p>
        </div>
      )}
    </div>
  );
};
export default GraffitiWallComponent;