import { useState } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";

interface Network {
  name: string;
  id: number;
}

interface IP{
  name: string;
  id: number;
}
function App() {
  const [Networks, setNetworks] = useState(Array<Network>());
  const [IPs, setIPs] = useState(Array<IP>());
  const [name, setName] = useState("");

  async function check_network_interfaces() {
    // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
    // console.log(await invoke<Array<String>>("list_network_interfaces"));
    setNetworks(await invoke<Array<Network>>("list_network_interfaces"));
  }

  async function list_ips(interface_name: String){
    // console.log(await invoke<Array<IP>>("list_ips",{interfaceName: interface_name}));
    setIPs(await invoke<Array<IP>>("list_ips",{interfaceName: interface_name}));
  }

  return (
    <div className="container">
      <h1>Welcome to Tauri!</h1>

      <div className="row">
        <a href="https://vitejs.dev" target="_blank">
          <img src="/vite.svg" className="logo vite" alt="Vite logo" />
        </a>
        <a href="https://tauri.app" target="_blank">
          <img src="/tauri.svg" className="logo tauri" alt="Tauri logo" />
        </a>
        <a href="https://reactjs.org" target="_blank">
          <img src={reactLogo} className="logo react" alt="React logo" />
        </a>
      </div>

      <p>Click on the Tauri, Vite, and React logos to learn more.</p>

      <div className="row">
        <form
          onClick={(e) => {
            e.preventDefault();
            check_network_interfaces();
          }}
        >
          {/*<input
            id="greet-input"
            onChange={(e) => setName(e.currentTarget.value)}
            placeholder="Enter a name..."
          />*/}
          <button type="button">Find network interfaces</button>
        </form>
      </div>
      <div>
        <form
          onClick={(e) => {
            e.preventDefault();
            console.log((e.target as HTMLInputElement).value);
            list_ips((e.target as HTMLInputElement).value);
          }}
        >
        {Networks.map(network => 
          <button type="button" key={network.id} value={network.name.toString()}>{network.name}</button>)
        }
        </form>
      </div>
      <div>
        {IPs.map(ip =>  
         <button type="button" key={ip.id} value={ip.name.toString()}>{ip.name}</button>)
        }
      </div>
    </div>
  );
}

export default App;
