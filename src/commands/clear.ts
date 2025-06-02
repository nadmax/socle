import { Command } from '../types/Command.js';
import { deleteMessagesConcurrently } from '../utils/deleteMessages.js';

import { ChatInputCommandInteraction, GuildMember, NewsChannel, SlashCommandBuilder, TextChannel, ThreadChannel } from 'discord.js';

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
        const roleId = process.env.MEMBER_ROLE_ID!;
        const role = interaction.guild?.roles.cache.get(roleId);
        if (!role) {
            await interaction.reply({
                content: '⚠️ Le rôle "Owner" est introuvable sur le serveur.',
                ephemeral: true,
            });
            return;
        }

        if (!member.roles.cache.has(role.id)) {
            await interaction.reply({
                content: `❌ Tu n'as pas les droits pour exécuter cette commande.`,
                ephemeral: true,
            });
            return;
        }


        if (amount < 1 || amount > 100) {
            await interaction.reply({
                content: '❌ Veuillez choisir un nombre entre 1 et 100.',
                ephemeral: true,
            });
            return;
        }

        const channel = interaction.channel;
        if (!channel ||!(channel instanceof TextChannel || channel instanceof NewsChannel || channel instanceof ThreadChannel)) {
            await interaction.reply({
                content: '❌ Cette commande ne peut être utilisée que dans un salon textuel.',
                ephemeral: true,
            });
            return;
        }

        await interaction.deferReply({ ephemeral: true });

        try {
            const fetchedMessages = await channel.messages.fetch({ limit: 100 });
            const deletedMessages = await channel.bulkDelete(fetchedMessages, true);
            const notDeleted = fetchedMessages.filter(
                msg => !deletedMessages.has(msg.id)
            );

            await deleteMessagesConcurrently(notDeleted.values(), 3, 300);

            await interaction.editReply(`✅ ${fetchedMessages.size} message(s) ont été supprimés avec succès.`);
        } catch (error) {
            console.error(error);
            await interaction.editReply('❌ Échec de la suppression des messages. Assurez-vous que les messages datent de moins de 14 jours.');
        }
    },
}