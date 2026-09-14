import { SettingsDialog } from './components/settings-dialog';
import { Preview } from './Preview';
import { Dashboard } from './Dashboard';
import { LiveAgentBar } from './LiveAgentBar';
import { useFleqi } from './backend';
import { nativeHost } from './native';
const surface=new URLSearchParams(location.search).get('surface');
document.documentElement.dataset.native=String(nativeHost);
document.documentElement.dataset.surface=surface==='bar'?'bar':surface==='preview'?'scene':surface==='settings'?'settings':'dashboard';
function Product(){const controller=useFleqi();return surface==='bar'?<><LiveAgentBar controller={controller}/>{!nativeHost&&<SettingsDialog controller={controller}/>}</>:<Dashboard controller={controller}/>;}
export function App(){return surface==='preview'?<Preview/>:<Product/>;}
