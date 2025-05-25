import { Client, Collection, Events, GatewayIntentBits } from 'discord.js';
import dotenv from 'dotenv';
import { command as ping } from './commands/ping.js';

dotenv.config();

const client = new Client({ intents: [GatewayIntentBits.Guilds] });
const commands = new Collection<string, typeof ping>();
commands.set(ping.data.name, ping);

client.once(Events.ClientReady, (c) => {
  console.log(`Ready! Logged in as ${c.user.tag}`);
});

client.on(Events.InteractionCreate, async (interaction) => {
  if (!interaction.isChatInputCommand()) return;

  const command = commands.get(interaction.commandName);
  if (!command) return;

  try {
    await command.execute(interaction);
  } catch (error) {
    console.error(error);
    await interaction.reply({ content: 'Error executing command.', ephemeral: true });
  }
});

client.login(process.env.DISCORD_TOKEN);