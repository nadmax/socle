import { ChatInputCommandInteraction, MessageFlags, SlashCommandBuilder } from 'discord.js';
import { Command } from '../types/Command.js';
import { commands } from './index.js';

export const help: Command = {
    data: new SlashCommandBuilder()
        .setName('help')
        .setDescription('Lists all available commands.'),

    async execute(interaction: ChatInputCommandInteraction) {
        const commandList = commands
            .map((cmd: { data: { name: any; description: any; }; }) => `**/${cmd.data.name}**: ${cmd.data.description}`)
            .join('\n');

        await interaction.reply({
            content: `📜 **Available Commands:**\n${commandList}`,
            flags: MessageFlags.Ephemeral,
        });
  },
};