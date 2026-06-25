import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const targetPath = path.join(__dirname, '..', 'component_test', 'stress.amana');

let content = `theme:
    preset: luxury
    radius: default
    density: default
    direction: ltr

route /stress -> view stress

view stress:
    render:
        Container:
            h1: "Compiler Stress Test"
`;

// Repeated 500 times: Grid(columns: "4") with 4 Cards
for (let i = 0; i < 500; i++) {
  content += `            Grid(columns: "4"):
                Card(title: "Card ${i} - 1", variant: "flat"):
                    p: "Content for card ${i}-1"
                Card(title: "Card ${i} - 2", variant: "elevated"):
                    p: "Content for card ${i}-2"
                Card(title: "Card ${i} - 3", variant: "outlined"):
                    p: "Content for card ${i}-3"
                Card(title: "Card ${i} - 4", variant: "glass"):
                    p: "Content for card ${i}-4"
`;
}

fs.writeFileSync(targetPath, content, 'utf8');
console.log(`Successfully generated stress.amana at ${targetPath}`);
