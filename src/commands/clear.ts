import { Command } from '../types/Command.js';
import { deleteMessagesConcurrently } from '../utils/deleteMessages.js';

import { ChatInputCommandInteraction, GuildMember, MessageFlags, NewsChannel, SlashCommandBuilder, TextChannel, ThreadChannel } from 'discord.js';

export const clear: Command = {
    data: new SlashCommandBuilder()
        .setName('clear')
        .setDescription('Deletes a given number of messages from the channel.')
        .addIntegerOption(option =>
            option
                .setName('amount')
                .setDescription('Number of messages to delete (1-100)')
                .setRequired(true)
        ),

    async execute(interaction: ChatInputCommandInteraction) {
        const amount = interaction.options.getInteger('amount')!;
        const member = interaction.member as GuildMember;
        const adminRoleId = process.env.ADMIN_ROLE_ID!;
        const adminRole = interaction.guild?.roles.cache.get(adminRoleId);
        if (!adminRole) {
            await interaction.reply({
                content: '⚠️"Admin" role cannot be found.',
                flags: MessageFlags.Ephemeral
            });
            return;
        }

        if (!member.roles.cache.has(adminRole.id)) {
            await interaction.reply({
                content: `❌ You do not have permission to execute this command.`,
                flags: MessageFlags.Ephemeral
            });
            return;
        }


        if (amount < 1 || amount > 100) {
            await interaction.reply({
                content: '❌ Please choose a number between 1 and 100.',
                flags: MessageFlags.Ephemeral
            });
            return;
        }

        const channel = interaction.channel;
        if (!channel || !(channel instanceof TextChannel || channel instanceof NewsChannel || channel instanceof ThreadChannel)) {
            await interaction.reply({
                content: '❌ This command can only be used in a text channel.',
                flags: MessageFlags.Ephemeral
            });
            return;
        }

        await interaction.deferReply({ flags: MessageFlags.Ephemeral });

        try {
            const fetchedMessages = await channel.messages.fetch({ limit: 100 });
            const deletedMessages = await channel.bulkDelete(fetchedMessages, true);
            const notDeleted = fetchedMessages.filter(
                msg => !deletedMessages.has(msg.id)
            );

            await deleteMessagesConcurrently(notDeleted.values(), 3, 300);

            if (fetchedMessages.size > 1)
                await interaction.editReply(`✅ ${fetchedMessages.size} messages were successfully deleted.`);
            else
                await interaction.editReply(`✅ 1 message was successfully deleted.`);
        } catch (error) {
            console.error(error);
            await interaction.editReply('❌ Failed to delete messages. Make sure messages are less than 14 days old.');
        }
    },
}
