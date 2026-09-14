import { createRoot } from 'react-dom/client';
import { TooltipProvider } from './components/ui/tooltip';
import { MotionProvider } from './motion';
import { installMotionOrigins } from './motion/origin';
import tokens from './motion/tokens.json';
import { App } from './App';
import './shadcn.css';
import './styles.css';
import './motion.css';

installMotionOrigins();
document.documentElement.style.setProperty('--motion-curve',`cubic-bezier(${tokens.curve.join(',')})`);
for(const [name,value] of Object.entries(tokens.duration))document.documentElement.style.setProperty(`--motion-${name}`,`${value}s`);

createRoot(document.getElementById('root')!).render(<MotionProvider><TooltipProvider><App /></TooltipProvider></MotionProvider>);
