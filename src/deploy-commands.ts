import { REST, Routes } from 'discord.js';
import dotenv from 'dotenv';
import { command as ping } from './commands/ping.js';

dotenv.config();

const commands = [ping.data.toJSON()];
const rest = new REST({ version: '10' }).setToken(process.env.DISCORD_TOKEN!);

(async () => {
    try {
        console.log('Deploying commands...');

        await rest.put(
            Routes.applicationGuildCommands(process.env.CLIENT_ID!, process.env.GUILD_ID!),
            { body: commands }
        );

        console.log('Commands deployed!');
    } catch (error) {
        console.error(error);
    }
})();