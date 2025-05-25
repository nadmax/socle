import { 
    type GuildMember, 
    EmbedBuilder, 
    ActionRowBuilder, 
    ButtonBuilder, 
    ButtonStyle, 
    Events
} from 'discord.js';
import { Event } from '../types/Event.js';

export const event: Event = {
    name: Events.GuildMemberAdd,
    once: false,

    async execute(member: GuildMember) {
        const welcomeChannelId = process.env.WELCOME_CHANNEL_ID!;
        const channel = member.guild.channels.cache.get(welcomeChannelId);
        if (!channel?.isTextBased()) {
            return;
        }

        const welcomeEmbed = new EmbedBuilder()
        .setTitle(`👋 Bienvenue ${member.user.username}!`)
        .setDescription('Clique sur le bouton ci-dessous pour accéder au serveur.')
        .setColor(0x00bfff);

        const button = new ButtonBuilder()
        .setCustomId(`welcome-role-${member.id}`)
        .setLabel('Accéder au serveur')
        .setStyle(ButtonStyle.Success);

        const row = new ActionRowBuilder<ButtonBuilder>().addComponents(button);

        await channel.send({
            content: `<@${member.id}>`,
            embeds: [welcomeEmbed],
            components: [row],
        });
    },
};
