import { clear } from './commands/clear.js';
import { ping } from './commands/ping.js';
import { Command } from './types/Command.js';

import { ActivityType, Client, Collection, Events, GatewayIntentBits } from 'discord.js';
import dotenv from 'dotenv';
import fs from 'fs';
import { fileURLToPath } from 'url';
import path from 'path';

dotenv.config();

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const client = new Client({
    intents: [
        GatewayIntentBits.Guilds,
        GatewayIntentBits.GuildMembers,
    ]
});
const commands = new Collection<string, Command>();

commands.set(ping.data.name, ping);
commands.set(clear.data.name, clear);
client.on(Events.InteractionCreate, async (interaction) => {
    if (!interaction.isChatInputCommand()) {
        return;
    }

    const command = commands.get(interaction.commandName);
    if (!command) {
        return;
    }

    try {
        await command.execute(interaction);
    } catch (error) {
        console.error(error);
        await interaction.reply({ content: 'Error executing command.', ephemeral: true });
    }
});

const eventsPath = path.join(__dirname, 'events');
const eventFiles = fs.readdirSync(eventsPath).filter(file => file.endsWith('.ts') || file.endsWith('.js'));

for (const file of eventFiles) {
    const filePath = path.join(eventsPath, file);
    const { event } = await import(filePath);

    if (!event) {
        console.warn(`⚠️ No 'event' export found in ${file}`);
        continue;
    }

    if (event.once) {
        client.once(event.name, (...args) => event.execute(...args));
    } else {
        client.on(event.name, (...args) => event.execute(...args));
    }
}

client.once(Events.ClientReady, (c) => {
    console.log(`Ready! Logged in as ${c.user.tag}`);
    c.user.setPresence({
        activities: [{
            name: 'Sync Status: Synced 🟢',
            type: ActivityType.Watching
        }],
        status: 'online'
    });
});
client.login(process.env.DISCORD_TOKEN);
