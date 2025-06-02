import { ChatInputCommandInteraction, SlashCommandBuilder } from 'discord.js';
import { Command } from '../types/Command.js';

export const ping: Command = {
    data: new SlashCommandBuilder()
        .setName('ping')
        .setDescription('Replies with bot latency and API latency.'),

    async execute(interaction: ChatInputCommandInteraction) {
        const sent = await interaction.reply({
            content: 'Pinging...',
            withResponse: true
        });

        const roundTrip = Date.now() - sent.interaction.createdTimestamp;
        const apiLatency = interaction.client.ws.ping;

        await interaction.editReply(
            `🏓 Pong!\nRound-trip latency: **${roundTrip}ms**\nWebSocket latency: **${apiLatency}ms**`
        );
    },
};