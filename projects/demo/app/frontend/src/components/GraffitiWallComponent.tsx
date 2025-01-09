import React, { useState, useEffect, useCallback } from 'react';
import { RpcConnection, MessageUtil, PubkeyUtil, Instruction, Message, UtxoMetaData } from '@saturnbtcio/arch-sdk';
import { Copy, Check, AlertCircle } from 'lucide-react';
import { Buffer } from 'buffer';
import { useWallet } from '../hooks/useWallet';
import * as borsh from 'borsh';
import { v4 as uuidv4 } from 'uuid';
import { request } from 'sats-connect';

// Configure global Buffer for browser environment
window.Buffer = Buffer;

// Environment variables for configuration
const client = new RpcConnection((import.meta as any).env.VITE_RPC_URL || 'http://localhost:9002');
const PROGRAM_PUBKEY = (import.meta as any).env.VITE_PROGRAM_PUBKEY;
const WALL_ACCOUNT_PUBKEY = (import.meta as any).env.VITE_WALL_ACCOUNT_PUBKEY;


const GraffitiWallComponent: React.FC = () => {
  // State management
  const wallet = useWallet();
  const [error, setError] = useState<string | null>(null);
  const [isAccountCreated, setIsAccountCreated] = useState(false);
  const [isProgramDeployed, setIsProgramDeployed] = useState(false);

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



  // Initialize component
  useEffect(() => {
    checkProgramDeployed();
    checkAccountCreated();
  }, [checkAccountCreated, checkProgramDeployed]);


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


  // Serialize create event instruction
  const serializeCreateEventInstruction = (
    unique_id: Uint8Array,
    expiry_timestamp: number,
    num_outcomes: number
  ): Uint8Array => {
    const schema = {
      struct: {
        function_number: 'u8',
        unique_id: { array: { type: 'u8', len: 32 } },
        expiry_timestamp: 'u32',
        num_outcomes: 'u8',
      }
    };


    const data = {
      function_number: 1,
      unique_id: Array.from(unique_id),
      expiry_timestamp,
      num_outcomes,
    };

    return borsh.serialize(schema, data);
  };

  const expiryTimestamp = 1689422272;
  const numOutcomes = 2;


  // Create event
  const handleCreateEvent = async () => {

    try {
      console.log("arr2:")
      const pubKeyUser: string = await window.unisat.getPublicKey();

      const uniqueId = new Uint8Array(32).fill(0); // Fill with your ID bytes
      const uniqueIdBytes = new TextEncoder().encode(uuidv4().replace("-", ""));
      uniqueId.set(uniqueIdBytes.slice(0, 32));

      
      const expiryTimestamp = 1689422272; // 24 hours from now
      const numOutcomes = 3;

      const data = serializeCreateEventInstruction(
        uniqueId,
        expiryTimestamp,
        numOutcomes
      );



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
            pubkey: PubkeyUtil.fromHex(pubKeyUser.slice(2,pubKeyUser.length)),
            is_signer: true,
            is_writable: false
          }
        ],
        data: data,
      };

      const messageObj: Message = {
        signers: [PubkeyUtil.fromHex(pubKeyUser.slice(2,pubKeyUser.length))],
        instructions: [instruction],
      };

      const messageHash = MessageUtil.hash(messageObj);
      // const signature = await wallet.signMessage(Buffer.from(messageHash).toString('hex'));
      let signature = await window.unisat.signMessage(Buffer.from(messageHash).toString('hex'));

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

      const pubKeyUser: string = await window.unisat.getPublicKey();
      const owner = new Uint8Array(32).fill(0); // Fill with your ID bytes
      const ownerBytes = new TextEncoder().encode(pubKeyUser.slice(2, pubKeyUser.length));
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
        function_number: 4,
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
        signers: [PubkeyUtil.fromHex(pubKeyUser.slice(2, pubKeyUser.length))],
        instructions: [instruction],
      };

      const messageHash = MessageUtil.hash(messageObj);
      // const signature = await wallet.signMessage(Buffer.from(messageHash).toString('hex'));
      let signature = await window.unisat.signMessage(Buffer.from(messageHash).toString('hex'));
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



  const serializeBetEventInstruction = (
    unique_id: Uint8Array,
    outcome_id: number,
    amount: number,
    tx_hex: string,
    utxo: UtxoMetaData
  ): Uint8Array => {

    const betSchema = {
      struct: {
        function_number: 'u8',
        unique_id: { array: { type: 'u8', len: 32 } },
        outcome_id: 'u8',
        amount: 'u64',
        tx_hex: { array: { type: 'u8' } },  // Variable length array of u8
        utxo: {
          struct: {
            txid: 'string',
            vout: 'u32'
          }
        }
      }
    };

    const data = {
      function_number: 3,
      unique_id: Array.from(unique_id), // Your 32-byte array
      outcome_id: outcome_id,
      amount: BigInt(amount),  // u64 needs to be BigInt
      tx_hex: tx_hex,
      utxo: utxo
    };

    return borsh.serialize(betSchema, data);
  };

  // Create event
  const handleBetEvent = async () => {
    if (!wallet.isConnected || !expiryTimestamp) {
      setError("Wallet must be connected and expiry time must be set");
      return;
    }

    try {
      const uniqueId = new Uint8Array(32).fill(0); // Fill with your ID bytes
      const uniqueIdBytes = new TextEncoder().encode("3e364ca1b9804049b39d71bfd2eee");
      uniqueId.set(uniqueIdBytes.slice(0, 32));


      const response = await request("sendTransfer", {
        recipients: [
          {
            address: "tb1p5vm3478tr966nysql5nls6wg3jr23puy9edlrjygnxsqcuyktehqeyzgxd",
            amount: Number(20000),
          },
        ],
      });

      // let txid = await window.unisat.sendBitcoin("tb1qrn7tvhdf6wnh790384ahj56u0xaa0kqgautnnz",1000);


      // console.log(txid);
      // return

      let utxo = {
        txid: "86e68158dea1986a3e5ed05d646265829f30cf406648b524f9ea9d65bd0be516",
        vout: 0
      }

      const data = serializeBetEventInstruction(
        uniqueId,
        1,
        100,
        utxo.txid,
        utxo
      );


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



  const handleCloseEvent = async () => {
    if (!wallet.isConnected) {
      setError("Wallet must be connected");
      return;
    }

    try {
      const uniqueId = new Uint8Array(32).fill(0); // Fill with your ID bytes
      const uniqueIdBytes = new TextEncoder().encode("dasdasdlkqwhjddsasdadadadadaaaaa");
      uniqueId.set(uniqueIdBytes.slice(0, 32));

      const schema = {
        struct: {
          function_number: 'u8',
          unique_id: { array: { type: 'u8', len: 32 } },
        }
      };

      let data = {
        function_number: 2,
        unique_id: Array.from(uniqueId),
      };

      let serialData = borsh.serialize(schema, data);

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
        data: serialData,
      };

      const messageObj: Message = {
        signers: [PubkeyUtil.fromHex(wallet.publicKey!)],
        instructions: [instruction],
      };

      const messageHash = MessageUtil.hash(messageObj);
      const signature = await wallet.signMessage(Buffer.from(messageHash).toString('hex'));
      const signatureBytes = new Uint8Array(Buffer.from(signature, 'base64')).slice(2);

      const result = await client.sendTransaction({
        version: 0,
        signatures: [signatureBytes],
        message: messageObj,
      });

      console.log(result);

    } catch (error) {
      console.error('Error creating event:', error);
      setError(`Failed to create event: ${error instanceof Error ? error.message : String(error)}`);
    }
  };


  const fetchEventData = useCallback(async () => {
    try {
      const account = await client.readAccountInfo(accountPubkey);

      if (!account) {
        setError('Account not found.');
        return;
      }

      const eventData = borsh.deserialize(
        {
          struct: {
            total_predictions: 'u32',
            predictions: {
              array: {
                type: {
                  struct: {
                    unique_id: { array: { type: 'u8', len: 32 } },
                    creator: { array: { type: 'u8', len: 32 } }, // Pubkey
                    expiry_timestamp: 'u32',
                    outcomes: {
                      array: {
                        type: {
                          struct: {
                            id: 'u8',
                            total_amount: 'u64',
                            bets: {
                              map: {
                                key: { array: { type: 'u8', len: 32 } }, // Pubkey as key
                                value: {
                                  array: {
                                    type: {
                                      struct: {
                                        user: { array: { type: 'u8', len: 32 } }, // Pubkey
                                        event_id: { array: { type: 'u8', len: 32 } },
                                        outcome_id: 'u8',
                                        amount: 'u64',
                                        tx_hex: { array: { type: 'u8' } }, // Vec<u8>
                                        utxo: {
                                          struct: {
                                            txid: { array: { type: 'u8', len: 32 } },
                                            vout: 'u32'
                                          }
                                        },
                                        timestamp: 'i64'
                                      }
                                    }
                                  }
                                }
                              }
                            }
                          }
                        }
                      }
                    },
                    total_pool_amount: 'u64',
                    status: 'u8', // EventStatus enum
                    winning_outcome: { option: 'u8' }
                  }
                }
              }
            }
          }
        },
        account.data
      );

      console.log(eventData)
    } catch (error) {
      console.error('Error fetching event data:', error);
      setError(`Failed to fetch event data: ${error instanceof Error ? error.message : String(error)}`);
    }
  }, []);



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
                onKeyDown={() => { }}
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
                disabled={!isFormValid}
              >
                Create Predition
              </button>
              <button
                onClick={handleBetEvent}
                className={`w-full font-bold py-2 px-4 rounded-lg transition duration-300 ${isFormValid
                  ? 'bg-arch-orange text-arch-black hover:bg-arch-white hover:text-arch-orange'
                  : 'bg-gray-500 text-gray-300 cursor-not-allowed'
                  }`}
              >
                Bet
              </button>
              <button
                onClick={handleCloseEvent}
                className={`w-full font-bold py-2 px-4 rounded-lg transition duration-300 ${isFormValid
                  ? 'bg-arch-orange text-arch-black hover:bg-arch-white hover:text-arch-orange'
                  : 'bg-gray-500 text-gray-300 cursor-not-allowed'
                  }`}
                disabled={!isFormValid}
              >
                Close Predition
              </button>

              <button
                onClick={handleCreateNewToken}
                className={`w-full font-bold py-2 px-4 rounded-lg transition duration-300 bg-arch-orange text-arch-black hover:bg-arch-white hover:text-arch-orange`}
              >
                Create New Token
              </button>

              <button
                onClick={fetchEventData}
                className={`w-full font-bold py-2 px-4 rounded-lg transition duration-300 bg-arch-orange text-arch-black hover:bg-arch-white hover:text-arch-orange`}
              >
                Fetch Event
              </button>
            </div>
          </div>

          <div className="flex-1">
            <div className="bg-arch-black p-6 rounded-lg">
              <h3 className="text-2xl font-bold mb-4 text-arch-white">Wall Messages</h3>
              <div className="space-y-4 max-h-96 overflow-y-auto">

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